//! Symlink tests: an `lstat` taken through a symlinked directory is reused for the real path.

use std::{
    env, fs, io,
    path::{MAIN_SEPARATOR, Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use oxc_resolver::{
    FileMetadata, FileSystem, FileSystemOs, Resolution, ResolveError, ResolveOptions, Resolver,
    ResolverGeneric,
};

/// The OS file system, reporting every path passed to `symlink_metadata`.
struct ObservedFs {
    on_lstat: Box<dyn Fn(&Path) + Send + Sync>,
}

impl FileSystem for ObservedFs {
    #[cfg(feature = "yarn_pnp")]
    fn new(_yarn_pnp: bool) -> Self {
        unreachable!()
    }

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        unreachable!()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        FileSystemOs::read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        FileSystemOs::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        (self.on_lstat)(path);
        FileSystemOs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        FileSystemOs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        FileSystemOs::canonicalize(path)
    }
}

/// A resolver over the OS file system, and the paths it has `lstat`ed.
fn lstat_recording_resolver() -> (ResolverGeneric<ObservedFs>, Arc<Mutex<Vec<PathBuf>>>) {
    let lstats = Arc::new(Mutex::new(vec![]));
    let record = {
        let lstats = Arc::clone(&lstats);
        move |path: &Path| lstats.lock().unwrap().push(path.to_path_buf())
    };
    let fs = ObservedFs { on_lstat: Box::new(record) };
    (ResolverGeneric::new_with_file_system(fs, ResolveOptions::default()), lstats)
}

/// `path` below `root` with native separators, which is how the resolver spells the paths it
/// derives.
fn at(root: &Path, path: &str) -> PathBuf {
    path.split('/').fold(root.to_path_buf(), |path, part| path.join(part))
}

/// The fixture `fixtures/<path>`, or `None` when its symlink `link` was not checked out as one.
fn symlink_fixture(path: &str, link: &str) -> Option<PathBuf> {
    let root = at(&env::current_dir().unwrap().join("fixtures"), path);
    if fs::symlink_metadata(at(&root, link)).is_ok_and(|m| m.is_symlink()) {
        return Some(root);
    }
    eprintln!("skip: symlinks unavailable in {}", root.display());
    None
}

/// Collects every file below `dir` under every spelling of it, in a fixed order, with whether the
/// spelling passes through a symlink. `walkdir` cannot follow links on WASI, and the fixtures
/// have no symlink cycles.
fn file_spellings(dir: &Path, through_symlink: bool, files: &mut Vec<(PathBuf, bool)>) {
    let mut paths: Vec<_> = fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().path()).collect();
    paths.sort();
    for path in paths {
        let through_symlink = through_symlink || path.symlink_metadata().unwrap().is_symlink();
        if path.is_dir() {
            file_spellings(&path, through_symlink, files);
        } else {
            files.push((path, through_symlink));
        }
    }
}

/// A path reached through a symlinked ancestor names the same directory entry as its real
/// spelling, so the `lstat`s issued on the way through the link also answer for the real location.
/// Resolving from there afterwards — which is where the imports of a linked or isolated package
/// come from — must not `lstat` those entries again.
#[test]
fn lstat_through_symlink_is_reused_for_real_path() {
    let fixture = "node-modules-canonicalize/symlinked-workspace";
    let Some(root) = symlink_fixture(fixture, "node_modules/pkg") else { return };
    let (resolver, lstats) = lstat_recording_resolver();
    let real_dir = at(&root, "packages/pkg/real");

    // Through the `node_modules/pkg -> ../packages/pkg` link.
    let resolution = resolver.resolve(&root, "pkg/real/file.js").unwrap();
    assert_eq!(resolution.path(), real_dir.join("file.js"));
    assert!(!lstats.lock().unwrap().is_empty());

    // From the real location: `real` and `real/file.js` were both seen through the link.
    lstats.lock().unwrap().clear();
    let resolution = resolver.resolve(&real_dir, "./file.js").unwrap();
    assert_eq!(resolution.path(), real_dir.join("file.js"));
    assert_eq!(*lstats.lock().unwrap(), Vec::<PathBuf>::new());
}

/// The `lstat` reused for a real spelling must be that entry's own. Whatever a resolver has seen
/// through symlinks, it resolves every file under every spelling as a resolver that has seen
/// nothing does.
#[test]
fn lstat_reused_for_real_path_agrees_with_fresh_resolver() {
    let link = "symlinked-workspace/node_modules/pkg";
    let Some(layouts) = symlink_fixture("node-modules-canonicalize", link) else { return };
    let Some(spelling) = symlink_fixture("integration/symlink-spelling", "link") else { return };
    let (resolver, lstats) = lstat_recording_resolver();
    let assert_agrees = |directory: &Path, specifier: &str| {
        let fresh = Resolver::default().resolve(directory, specifier);
        assert_eq!(
            resolver.resolve(directory, specifier).map(Resolution::into_path_buf),
            fresh.map(Resolution::into_path_buf),
            "{specifier} from {}",
            directory.display()
        );
    };

    // A trailing separator or `.` makes the OS follow the final symlink, so the `lstat` of these
    // spellings describes the target directory rather than the `link` entry. Looking up the
    // package that a bare specifier is imported from canonicalizes the directory as spelled.
    let link = spelling.join("link");
    assert_agrees(Path::new(&format!("{}{MAIN_SEPARATOR}", link.display())), "pkg");
    assert_agrees(Path::new(&format!("{}{MAIN_SEPARATOR}.", link.display())), "pkg");

    // The spellings that pass through a symlink go first, so that nothing below a link target is
    // `lstat`ed under its real spelling before it is resolved from there.
    let mut files = vec![];
    file_spellings(&layouts, false, &mut files);
    file_spellings(&spelling, false, &mut files);
    for through_symlink in [true, false] {
        for (path, _) in files.iter().filter(|(_, through)| *through == through_symlink) {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert_agrees(path.parent().unwrap(), &format!("./{name}"));
        }
    }

    // Entries that are only reachable below a link target were answered by the first round.
    for path in [
        "symlinked-workspace/packages/pkg/real/file.js",
        "internal-symlink/node_modules/pkg/real/file.js",
        "nested-monorepo/node_modules/.pnpm/dep@1.0.0/node_modules/dep/index.js",
    ] {
        assert!(!lstats.lock().unwrap().contains(&at(&layouts, path)), "{path} was not reused");
    }
}

/// `..` is where the resolver and the OS can disagree about the directory a path names. Inside a
/// symlink target the resolver folds it lexically, while POSIX follows the preceding component
/// first; in a path, the resolver applies it to the link target, while Windows folds it lexically.
/// An `lstat` taken through such a path must not be reused for the real spelling derived from it.
#[test]
fn lstat_is_not_reused_past_parent_dir() {
    let Some(root) = symlink_fixture("integration/symlink-spelling", "hop") else { return };
    let resolver = Resolver::new(ResolveOptions::default());

    // `hop -> sub/inner/../data`, where `sub/inner -> ../elsewhere/deep`: the file is
    // `elsewhere/data/f.js`, and `sub/data` does not exist.
    _ = resolver.resolve(root.join("hop"), "./f.js");
    assert!(matches!(
        resolver.resolve(at(&root, "sub/data"), "./f.js"),
        Err(ResolveError::NotFound(_))
    ));

    // `link -> store/pkg`: `link/../a.js` is `store/a.js` on POSIX, which does not exist, and
    // `a.js` on Windows, which does.
    _ = resolver.resolve(at(&root, "link/.."), "./a.js");
    assert!(matches!(
        resolver.resolve(root.join("store"), "./a.js"),
        Err(ResolveError::NotFound(_))
    ));
}

/// A resolve that is still running when `clear_cache` is called holds entries from before the
/// clear, and must not hand an `lstat` cached on them to an entry created after it. The file
/// system clears the cache in the middle of a resolve here, standing in for another thread.
#[test]
fn lstat_from_before_clear_is_not_reused() {
    let fixture = "node-modules-canonicalize/symlinked-workspace";
    let Some(root) = symlink_fixture(fixture, "node_modules/pkg") else { return };
    let through_link = at(&root, "node_modules/pkg/real/file.js");
    let real = at(&root, "packages/pkg/real/file.js");

    let lstats = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
    let handle = Arc::new(OnceLock::<Weak<ResolverGeneric<ObservedFs>>>::new());
    let cleared = Arc::new(AtomicBool::new(false));
    let clear_after_file = {
        let (lstats, handle, cleared) =
            (Arc::clone(&lstats), Arc::clone(&handle), Arc::clone(&cleared));
        move |path: &Path| {
            let is_after_file = {
                let mut lstats = lstats.lock().unwrap();
                let is_after_file = lstats.last() == Some(&through_link);
                lstats.push(path.to_path_buf());
                is_after_file
            };
            // The file has its `lstat`, and has not been canonicalized to its real path yet.
            if is_after_file && !cleared.swap(true, Ordering::Relaxed) {
                let resolver = handle.get().unwrap().upgrade().unwrap();
                resolver.clear_cache();
                // The same key interned again is a different entry, and does not vouch for the
                // one from before the clear.
                _ = resolver.resolve(&through_link, "./x");
            }
        }
    };
    let fs = ObservedFs { on_lstat: Box::new(clear_after_file) };
    let resolver = Arc::new(ResolverGeneric::new_with_file_system(fs, ResolveOptions::default()));
    handle.set(Arc::downgrade(&resolver)).unwrap();

    assert_eq!(resolver.resolve(&root, "pkg/real/file.js").unwrap().path(), real);
    assert!(cleared.load(Ordering::Relaxed));

    // The real path is a new entry, so it is `lstat`ed.
    lstats.lock().unwrap().clear();
    assert_eq!(resolver.resolve(real.parent().unwrap(), "./file.js").unwrap().path(), real);
    assert!(lstats.lock().unwrap().contains(&real));
}
