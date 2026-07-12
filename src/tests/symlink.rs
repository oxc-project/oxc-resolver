#[cfg(target_os = "windows")]
use crate::PathUtil;
#[cfg(target_os = "windows")]
use crate::tests::windows::get_dos_device_path;
use std::path::PathBuf;
use std::{fs, io, path::Path};

use walkdir::WalkDir;

use super::fixture_root;
use crate::{ResolveOptions, Resolver};

#[derive(Debug, Clone, Copy)]
enum FileType {
    File,
    Dir,
}

#[cfg_attr(
    not(target_os = "windows"),
    expect(unused_variables, reason = "`file_type` is only used on Windows")
)]
fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    original: P,
    link: Q,
    file_type: FileType,
) -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        std::os::unix::fs::symlink(original, link)
    }

    #[cfg(target_os = "windows")]
    match file_type {
        // NOTE: original path should use `\` instead of `/` for relative paths
        //       otherwise the symlink will be broken and the test will fail with InvalidFilename error
        FileType::File => std::os::windows::fs::symlink_file(original.as_ref().normalize(), link),
        FileType::Dir => std::os::windows::fs::symlink_dir(original.as_ref().normalize(), link),
    }
    #[cfg(target_family = "wasm")]
    {
        Err(io::Error::new(io::ErrorKind::Other, "not supported"))
    }
}

fn init(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    if temp_path.exists() {
        _ = fs::remove_dir_all(temp_path);
    }
    fs::create_dir(temp_path)?;
    symlink(dirname.join("../lib/index.js"), temp_path.join("test"), FileType::File)?;
    symlink(dirname.join("../lib"), temp_path.join("test2"), FileType::Dir)?;
    fs::remove_dir_all(temp_path)
}

fn create_symlinks(dirname: &Path, temp_path: &Path) -> io::Result<()> {
    fs::create_dir(temp_path)?;
    symlink(
        dirname.join("../lib/index.js").canonicalize()?,
        temp_path.join("index.js"),
        FileType::File,
    )?;
    symlink(dirname.join("../lib").canonicalize().unwrap(), temp_path.join("lib"), FileType::Dir)?;
    symlink(dirname.join("..").canonicalize().unwrap(), temp_path.join("this"), FileType::Dir)?;
    symlink(temp_path.join("this"), temp_path.join("that"), FileType::Dir)?;
    symlink(Path::new("../../lib/index.js"), temp_path.join("node.relative.js"), FileType::File)?;
    symlink(
        Path::new("./node.relative.js"),
        temp_path.join("node.relative.sym.js"),
        FileType::File,
    )?;

    #[cfg(target_os = "windows")]
    {
        // Ideally we should point to a Volume that does not have a drive letter.
        // However, it's not trivial to create a Volume in CI environment.
        // Here we are just picking up any Volume, as resolver itself is not calling `fs::canonicalize`,
        // which potentially can resolve the Volume GUID into driver letter whenever possible.
        let dos_device_temp_path = get_dos_device_path(temp_path).unwrap();
        symlink(
            dos_device_temp_path.join(r"..\..\lib"),
            temp_path.join("device_path_lib"),
            FileType::Dir,
        )?;
        symlink(
            dos_device_temp_path.join(r"..\..\lib\index.js"),
            temp_path.join("device_path_index.js"),
            FileType::File,
        )?;
    }

    Ok(())
}

fn cleanup_symlinks(temp_path: &Path) {
    _ = fs::remove_dir_all(temp_path);
}

struct SymlinkFixturePaths {
    root: PathBuf,
    temp_path: PathBuf,
}

/// Prepares symlinks for the test.
/// Specify a different `temp_path_segment` for each test to avoid conflicts when tests are executed concurrently.
/// Returns `Ok(None)` if the symlink fixtures cannot be created at all (usually due to a lack of permission).
/// Returns `Ok(Some(_))` if the symlink fixtures are created successfully, or already exist.
/// Returns `Err(_)` if there is error creating the symlinks.
fn prepare_symlinks<P: AsRef<Path>>(
    temp_path_segment: P,
) -> io::Result<Option<SymlinkFixturePaths>> {
    let root = super::fixture_root().join("enhanced-resolve");
    let dirname = root.join("test");
    let temp_path = dirname.join(temp_path_segment.as_ref());
    if !temp_path.exists() {
        if let Err(err) = init(&dirname, &temp_path) {
            println!(
                "Skipped test: Failed to create symlinks. You may need administrator privileges. Error: {err}"
            );
            return Ok(None);
        }
        if let Err(err) = create_symlinks(&dirname, &temp_path) {
            cleanup_symlinks(&temp_path);
            return Err(err);
        }
    }

    Ok(Some(SymlinkFixturePaths { root, temp_path }))
}

#[test]
#[cfg_attr(target_family = "wasm", ignore)]
fn test() {
    let Some(SymlinkFixturePaths { root, temp_path }) = prepare_symlinks("temp").unwrap() else {
        return;
    };
    let resolver_without_symlinks =
        Resolver::new(ResolveOptions { symlinks: false, ..ResolveOptions::default() });
    let resolver_with_symlinks = Resolver::default();

    #[rustfmt::skip]
    let pass = [
        ("with a symlink to a file", temp_path.clone(), "./index.js"),
        ("with a relative symlink to a file", temp_path.clone(), "./node.relative.js"),
        ("with a relative symlink to a symlink to a file", temp_path.clone(), "./node.relative.sym.js"),
        ("with a symlink to a directory 1", temp_path.clone(), "./lib/index.js"),
        ("with a symlink to a directory 2", temp_path.clone(), "./this/lib/index.js"),
        ("with multiple symlinks in the path 1", temp_path.clone(), "./this/test/temp/index.js"),
        ("with multiple symlinks in the path 2", temp_path.clone(), "./this/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3", temp_path.clone(), "./this/test/temp/this/lib/index.js"),
        ("with a symlink to a directory 2 (chained)", temp_path.clone(), "./that/lib/index.js"),
        ("with multiple symlinks in the path 1 (chained)", temp_path.clone(), "./that/test/temp/index.js"),
        ("with multiple symlinks in the path 2 (chained)", temp_path.clone(), "./that/test/temp/lib/index.js"),
        ("with multiple symlinks in the path 3 (chained)", temp_path.clone(), "./that/test/temp/that/lib/index.js"),
        ("with symlinked directory as context 1", temp_path.join( "lib"), "./index.js"),
        ("with symlinked directory as context 2", temp_path.join( "this"), "./lib/index.js"),
        ("with symlinked directory as context and in path", temp_path.join( "this"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path", temp_path.join( "this/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file", temp_path.join( "this/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory", temp_path.join( "this/test"), "./temp/lib/index.js"),
        ("with symlinked directory as context 2 (chained)", temp_path.join( "that"), "./lib/index.js"),
        ("with symlinked directory as context and in path (chained)", temp_path.join( "that"), "./test/temp/lib/index.js"),
        ("with symlinked directory in context path (chained)", temp_path.join( "that/lib"), "./index.js"),
        ("with symlinked directory in context path and symlinked file (chained)", temp_path.join( "that/test"), "./temp/index.js"),
        ("with symlinked directory in context path and symlinked directory (chained)", temp_path.join( "that/test"), "./temp/lib/index.js")
    ];

    for (comment, path, request) in pass {
        let filename = resolver_with_symlinks.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(filename, Ok(root.join("lib/index.js")), "{comment:?}");

        let resolved_path =
            resolver_without_symlinks.resolve(&path, request).map(|r| r.full_path());
        assert_eq!(resolved_path, Ok(path.join(request)));
    }
}

#[cfg(target_os = "windows")]
#[test]
fn test_unsupported_targets() {
    use crate::ResolveError;

    let Some(SymlinkFixturePaths { root, temp_path }) =
        prepare_symlinks("temp.test_unsupported_targets").unwrap()
    else {
        return;
    };
    let resolver_with_symlinks = Resolver::default();

    // Symlinks pointing to unsupported DOS device paths are not followed, as if `symlinks = false`.
    // See doc of `ResolveOptions::symlinks` for details.
    // They are treated as if they are ordinary files and folders.
    // FIXME: these tests does no pass
    // assert_eq!(
    //     resolver_with_symlinks.resolve(&temp_path, "./device_path_lib").unwrap().full_path(),
    //     temp_path.join("device_path_lib/index.js"),
    // );
    // assert_eq!(
    //     resolver_with_symlinks.resolve(&temp_path, "./device_path_index.js").unwrap().full_path(),
    //     temp_path.join("device_path_index.js"),
    // );

    // UB if the resolution starts at a directory with unsupported DOS device path. Don't do this.
    // While we haven't set up any convention on this, de facto behavior for now is
    // * if there is `package.json` in the ancestor, a `ResolveError::PathNotSupported` will be returned
    //   from `FsCachedPath::find_package_json` when trying to canonicalize the full path of `package.json`.
    // * Otherwise, a `ResolveError::NotFound` will be returned.
    let dos_device_temp_path = get_dos_device_path(&temp_path).unwrap();
    let dos_device_root = get_dos_device_path(&root).unwrap();
    assert_eq!(
        resolver_with_symlinks.resolve(&dos_device_temp_path, "./index.js"),
        Err(ResolveError::PathNotSupported(dos_device_root))
    );
}

#[test]
fn test_circular_symlink() {
    let Some(SymlinkFixturePaths { root: _, temp_path }) =
        prepare_symlinks("temp.test_circular_symlink").unwrap()
    else {
        return;
    };

    // Create a circular symlink: link1 -> link2 -> link1
    let link1_path = temp_path.join("link1");
    let link2_path = temp_path.join("link2");

    if symlink(&link2_path, &link1_path, FileType::File).is_err() {
        // Skip test if we can't create symlinks
        return;
    }
    if symlink(&link1_path, &link2_path, FileType::File).is_err() {
        // Skip test if we can't create symlinks
        _ = fs::remove_file(&link1_path);
        return;
    }

    let resolver = Resolver::default();
    let result = resolver.resolve(&temp_path, "./link1");

    // Should error due to circular symlink
    result.unwrap_err();

    // Cleanup
    _ = fs::remove_file(&link1_path);
    _ = fs::remove_file(&link2_path);
}

// ----------------------------------------------------------------------------
// `node_modules` canonicalization: assert the resolver's canonicalization equals
// `std::fs::canonicalize` across package-manager layouts and symlink shapes.
// ----------------------------------------------------------------------------

const COMBOS: &[&str] = &[
    "npm-flat",
    "pnpm-isolated",
    "pnpm-hoisted",
    "yarn-flat",
    "yarn-isolated",
    "yarn-pnp",
    "bun-flat",
    "bun-isolated",
];

/// Walk every entry under each installed `fixtures/bench-pm/installs/<combo>/node_modules` tree
/// (all npm/pnpm/yarn/bun × flat/isolated/hoisted/pnp layouts) and assert the resolver's
/// canonicalization equals `std::fs::canonicalize`, including the symlinks (and, on Windows,
/// junctions) into isolated virtual stores and the `.bin` shim directory. The fixtures are produced
/// by `just install-bench-fixtures` and are not committed, so combos that are not installed are
/// skipped.
#[test]
fn canonicalize_matches_os_for_all_node_modules() {
    let installs = fixture_root().join("bench-pm").join("installs");
    let mut combos_checked = 0u32;
    let mut paths_checked = 0u32;

    for combo in COMBOS {
        let node_modules = installs.join(combo).join("node_modules");
        if !node_modules.is_dir() {
            continue;
        }
        combos_checked += 1;
        let resolver = Resolver::new(ResolveOptions::default());

        for entry in WalkDir::new(&node_modules).follow_links(false) {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            // Ground truth from the OS. Skip broken symlinks / unreadable entries.
            let Ok(expected) = fs::canonicalize(path) else { continue };
            // `std::fs::canonicalize` returns a `\\?\` verbatim prefix on Windows; the resolver
            // strips it (`strip_windows_prefix`), so run the oracle through the same strip to
            // compare the two in the same representation.
            #[cfg(target_os = "windows")]
            let Ok(expected) = crate::windows::strip_windows_prefix(expected) else { continue };
            let cached = resolver.cache.value(path);
            let actual = resolver.cache.canonicalize(&cached).unwrap_or_else(|err| {
                panic!("{combo}: resolver canonicalize({}) failed: {err}", path.display())
            });
            assert_eq!(actual, expected, "{combo}: canonicalize mismatch for {}", path.display());
            paths_checked += 1;
        }
    }

    if combos_checked == 0 {
        eprintln!(
            "skip canonicalize_matches_os_for_all_node_modules: no bench-pm installs found \
             (run `just install-bench-fixtures`)"
        );
        return;
    }
    eprintln!("checked {paths_checked} paths across {combos_checked} bench-pm combos");
}

#[test]
fn canonicalize_dirty_cache_keys() {
    let f = super::fixture();
    let sep = std::path::MAIN_SEPARATOR;
    let mut dirty = vec![
        f.join(".."),
        f.join("lib").join(".."),
        f.join("lib").join("..").join("lib").join("complex1.js"),
        f.join("lib").join(".").join("complex1.js"),
        PathBuf::from(format!("{}{sep}", f.join("lib").display())),
        PathBuf::from(format!("{}{sep}{sep}complex1.js", f.join("lib").display())),
    ];
    #[cfg(unix)]
    {
        dirty.push(PathBuf::from(format!("{sep}{}", f.display())));
        dirty.push(PathBuf::from(format!("{sep}{sep}{}", f.display())));
    }
    #[cfg(target_os = "windows")]
    {
        dirty.push(PathBuf::from(f.display().to_string().replacen(":\\", ":\\\\", 1)));
    }
    let resolver = Resolver::new(ResolveOptions::default());
    for path in dirty {
        let expected = fs::canonicalize(&path).unwrap();
        #[cfg(target_os = "windows")]
        let expected = crate::windows::strip_windows_prefix(expected).unwrap();
        let cached = resolver.cache.value(&path);
        let actual = resolver.cache.canonicalize(&cached).unwrap();
        assert_eq!(actual.as_os_str(), expected.as_os_str(), "{}", path.display());
    }
}

/// A symlinked workspace package anchor with a symlink in the suffix below it: canonicalization must
/// follow both the anchor link and the inner link.
#[test]
fn symlinked_package_anchor_walks_suffix_symlinks() {
    let root = fixture_root().join("node_modules-canonicalize/symlinked-workspace");
    let anchor = root.join("node_modules/pkg");
    let inner_link = anchor.join("src/link");

    if !fs::symlink_metadata(&anchor).is_ok_and(|m| m.is_symlink())
        || !fs::symlink_metadata(&inner_link).is_ok_and(|m| m.is_symlink())
    {
        eprintln!("skip symlinked_package_anchor_walks_suffix_symlinks: symlinks unavailable");
        return;
    }

    let path = inner_link.join("file.js");
    let expected = fs::canonicalize(&path).unwrap();
    #[cfg(target_os = "windows")]
    let expected = crate::windows::strip_windows_prefix(expected).unwrap();

    let resolver = Resolver::new(ResolveOptions::default());
    let cached = resolver.cache.value(&path);
    let actual = resolver.cache.canonicalize(&cached).unwrap();

    assert_eq!(actual, expected);
    assert_eq!(expected, root.join("packages/pkg/real/file.js"));
}

/// A package in a recognized (lockfile-marked) flat layout can still ship a symlink below its real
/// `node_modules/<pkg>` anchor — a directory link (`lib -> dist`) or a re-export file link.
/// Canonicalization must follow it, not append the suffix across it.
#[test]
fn real_package_anchor_walks_internal_symlinks() {
    let root = fixture_root().join("node_modules-canonicalize/internal-symlink");
    let dir_link = root.join("node_modules/pkg/lib");
    let file_link = root.join("node_modules/pkg/reexport.js");

    if !fs::symlink_metadata(&dir_link).is_ok_and(|m| m.is_symlink())
        || !fs::symlink_metadata(&file_link).is_ok_and(|m| m.is_symlink())
    {
        eprintln!("skip real_package_anchor_walks_internal_symlinks: symlinks unavailable");
        return;
    }

    let real = root.join("node_modules/pkg/real/file.js");
    let resolver = Resolver::new(ResolveOptions::default());
    for path in [dir_link.join("file.js"), file_link] {
        let expected = fs::canonicalize(&path).unwrap();
        #[cfg(target_os = "windows")]
        let expected = crate::windows::strip_windows_prefix(expected).unwrap();
        let cached = resolver.cache.value(&path);
        let actual = resolver.cache.canonicalize(&cached).unwrap();
        assert_eq!(actual, expected, "{}", path.display());
        assert_eq!(expected, real, "{}", path.display());
    }
}

/// A monorepo with a version conflict: the root resolves `dep@2` while a workspace package nests
/// its own `dep@1` (an isolated-store symlink). Canonicalization must pick each package's own
/// deepest `node_modules/<pkg>` anchor and follow it to the correct store version. Every path is
/// compared against `std::fs::canonicalize`, walking through symlinked directories too.
#[test]
fn nested_monorepo_canonicalize_matches_os() {
    let root = fixture_root().join("node_modules-canonicalize/nested-monorepo");
    let nested = root.join("packages/ui/node_modules/dep");
    if !fs::symlink_metadata(&nested).is_ok_and(|m| m.is_symlink()) {
        eprintln!("skip nested_monorepo_canonicalize_matches_os: symlinks unavailable");
        return;
    }

    let resolver = Resolver::new(ResolveOptions::default());
    let walk = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .chain(WalkDir::new(&root).follow_links(true).max_depth(12));
    for entry in walk {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let Ok(expected) = fs::canonicalize(path) else { continue };
        #[cfg(target_os = "windows")]
        let Ok(expected) = crate::windows::strip_windows_prefix(expected) else { continue };
        let cached = resolver.cache.value(path);
        let actual = resolver.cache.canonicalize(&cached).unwrap();
        assert_eq!(actual, expected, "canonicalize mismatch for {}", path.display());
    }

    // The conflicting versions resolve to their respective stores.
    let resolve = |p: &Path| {
        let cached = resolver.cache.value(p);
        resolver.cache.canonicalize(&cached).unwrap()
    };
    assert_eq!(
        resolve(&nested.join("index.js")),
        root.join("node_modules/.pnpm/dep@1.0.0/node_modules/dep/index.js")
    );
    assert_eq!(
        resolve(&root.join("node_modules/dep/index.js")),
        root.join("node_modules/.pnpm/dep@2.0.0/node_modules/dep/index.js")
    );
}
