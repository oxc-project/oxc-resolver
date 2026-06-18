//! Filesystem-call-count snapshot for oxc-resolver.
//!
//! Wraps the [`FileSystem`] trait in a counting decorator (`CountingFs`) and tallies the per-method
//! *logical* call counts the resolver issues, then writes a committed `fs.snap`. CI runs
//! `cargo fs-calls` and fails the PR on `git diff`, so any change to the resolver's filesystem
//! traffic (a `stat` storm, an extra `package.json` read, a redundant probe) is a reviewable diff.
//!
//! Determinism / parity (local == CI):
//! - The workload runs **single-threaded** over a fixed **in-memory** filesystem rooted at `/`, so
//!   ancestor `node_modules` / `package.json` walks terminate at a fixed depth, independent of the
//!   real checkout path.
//! - `NODE_PATH` and `OXC_RESOLVER_YARN_PNP` are cleared at startup; both feed process-wide state
//!   (an `OnceLock` / a default option) that would otherwise perturb the walk on some CI runners.
//! - Every iterated `exports` / `imports` / condition / `browser` object stays **<= 32 keys**: above
//!   that, halfbrown switches to a foldhash-randomized HashMap whose iteration order is per-process
//!   random, which would change *which* target wins a first-match and thus the fs traffic. A startup
//!   assertion (`assert_objects_within_32`) enforces this on every fixture so it can never silently
//!   regress.
//!
//! Logical trait-call counts do not depend on the allocator, core count, or SIMD.
//!
//! Out of scope (need separate infra, tracked as follow-ups): symlink / pnpm realpath
//! (`read_link` / `canonicalize`), yarn-pnp, and `.d.ts` (`resolve_dts`).

use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
};

use oxc_resolver::{
    AliasValue, FileMetadata, FileSystem, ResolveContext, ResolveError, ResolveOptions,
    ResolverGeneric, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

/// Per-method logical fs-call counts (one field per `FileSystem` trait method).
#[derive(Default)]
struct FsCounts {
    read: AtomicUsize,
    read_to_string: AtomicUsize,
    metadata: AtomicUsize,
    symlink_metadata: AtomicUsize,
    read_link: AtomicUsize,
    canonicalize: AtomicUsize,
}

impl FsCounts {
    fn reset(&self) {
        self.read.store(0, Relaxed);
        self.read_to_string.store(0, Relaxed);
        self.metadata.store(0, Relaxed);
        self.symlink_metadata.store(0, Relaxed);
        self.read_link.store(0, Relaxed);
        self.canonicalize.store(0, Relaxed);
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            read: self.read.load(Relaxed),
            read_to_string: self.read_to_string.load(Relaxed),
            metadata: self.metadata.load(Relaxed),
            symlink_metadata: self.symlink_metadata.load(Relaxed),
            read_link: self.read_link.load(Relaxed),
            canonicalize: self.canonicalize.load(Relaxed),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Snapshot {
    read: usize,
    read_to_string: usize,
    metadata: usize,
    symlink_metadata: usize,
    read_link: usize,
    canonicalize: usize,
}

/// A `FileSystem` decorator that counts each logical call then forwards to the inner FS.
struct CountingFs<Fs: FileSystem> {
    inner: Fs,
    counts: Arc<FsCounts>,
}

impl<Fs: FileSystem> CountingFs<Fs> {
    fn wrap(inner: Fs, counts: Arc<FsCounts>) -> Self {
        Self { inner, counts }
    }
}

impl<Fs: FileSystem> FileSystem for CountingFs<Fs> {
    // `yarn_pnp` is always enabled for this crate (see Cargo.toml), so the constructor takes a bool.
    fn new(yarn_pnp: bool) -> Self {
        Self::wrap(Fs::new(yarn_pnp), Arc::new(FsCounts::default()))
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.counts.read.fetch_add(1, Relaxed);
        self.inner.read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.counts.read_to_string.fetch_add(1, Relaxed);
        self.inner.read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.counts.metadata.fetch_add(1, Relaxed);
        self.inner.metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.counts.symlink_metadata.fetch_add(1, Relaxed);
        self.inner.symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        self.counts.read_link.fetch_add(1, Relaxed);
        self.inner.read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.counts.canonicalize.fetch_add(1, Relaxed);
        self.inner.canonicalize(path)
    }
}

/// An in-memory `FileSystem` backed by the `vfs` crate, rooted at `/` (ported from the crate's
/// test helper). No symlink support, so `read_link` errors and `canonicalize` is identity.
#[derive(Default)]
struct MemoryFs {
    fs: vfs::MemoryFS,
}

impl MemoryFs {
    fn with_files(files: &[(&str, &str)]) -> Self {
        let mut memory = Self { fs: vfs::MemoryFS::default() };
        for (path, content) in files {
            memory.add_file(Path::new(path), content);
        }
        memory
    }

    fn add_file(&mut self, path: &Path, content: &str) {
        use vfs::FileSystem as _;
        let fs = &mut self.fs;
        for ancestor in path.ancestors().collect::<Vec<_>>().iter().rev() {
            let ancestor = ancestor.to_string_lossy();
            if !fs.exists(ancestor.as_ref()).unwrap() {
                fs.create_dir(ancestor.as_ref()).unwrap();
            }
        }
        let mut file = fs.create_file(path.to_string_lossy().as_ref()).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

impl FileSystem for MemoryFs {
    fn new(_yarn_pnp: bool) -> Self {
        Self::default()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        use vfs::FileSystem as _;
        let mut file = self
            .fs
            .open_file(path.to_string_lossy().as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        use vfs::FileSystem as _;
        let metadata = self
            .fs
            .metadata(path.to_string_lossy().as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?;
        let is_file = metadata.file_type == vfs::VfsFileType::File;
        let is_dir = metadata.file_type == vfs::VfsFileType::Directory;
        Ok(FileMetadata::new(is_file, is_dir, false))
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata(path)
    }

    fn read_link(&self, _path: &Path) -> Result<PathBuf, ResolveError> {
        Err(io::Error::new(io::ErrorKind::NotFound, "not a symlink").into())
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        use vfs::FileSystem as _;
        self.fs
            .metadata(path.to_string_lossy().as_ref())
            .map_err(|err| io::Error::new(io::ErrorKind::NotFound, err))?;
        Ok(path.to_path_buf())
    }
}

// ---- fixtures (all JSON objects <= 32 keys; enforced by assert_objects_within_32 at startup) ----

const PKG_ROOT: &str = r#"{ "name": "root", "version": "0.0.0", "type": "commonjs" }"#;
const TSCONFIG: &str = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": { "@app/*": ["./src/*"], "@lib": ["./lib/index.ts"] },
    "target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler"
  }
}"#;
// exports map: ".", "./sub", "./cond", "./features/*" = 4 keys (<= 32).
const PKG_PKG: &str = r#"{
  "name": "pkg", "version": "1.0.0",
  "exports": {
    ".": "./index.js",
    "./sub": "./sub.js",
    "./cond": { "import": "./c.mjs", "require": "./c.cjs", "default": "./c.js" },
    "./features/*": "./feat/*.js"
  }
}"#;
const PKG_SCOPE: &str = r#"{ "name": "@scope/comp", "version": "1.0.0", "main": "./main.js" }"#;
const PKG_IMP: &str = r##"{
  "name": "imp", "version": "1.0.0",
  "exports": { ".": "./index.js" },
  "imports": {
    "#internal": "./internal.js",
    "#cond": { "node": "./n.js", "default": "./d.js" }
  }
}"##;
const PKG_BROW: &str = r#"{
  "name": "brow", "version": "1.0.0", "main": "./main.js",
  "browser": { "./main.js": "./browser.js" }
}"#;

/// Assert every nested JSON object has <= 32 keys (halfbrown insertion-order bound). Panics with a
/// helpful message on violation so a future fixture edit can never silently introduce flake.
fn assert_objects_within_32(label: &str, json: &str) {
    fn walk(label: &str, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                assert!(
                    map.len() <= 32,
                    "{label}: a JSON object has {} keys (> 32) — halfbrown would switch to \
                     randomized-iteration HashMap mode and break fs-call determinism",
                    map.len()
                );
                for child in map.values() {
                    walk(label, child);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    walk(label, child);
                }
            }
            _ => {}
        }
    }
    let value: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|err| panic!("{label}: invalid JSON: {err}"));
    walk(label, &value);
}

/// Build the fixed in-memory project tree under `/p`.
fn build_fs() -> MemoryFs {
    MemoryFs::with_files(&[
        // project root + relative / directory / extensions-fallback targets
        ("/p/package.json", PKG_ROOT),
        ("/p/tsconfig.json", TSCONFIG),
        ("/p/index.js", ""),
        ("/p/a.js", ""),
        ("/p/foo.json", ""), // extensions fallback: ./foo misses .ts/.js, hits .json
        ("/p/dir/index.js", ""), // directory -> index
        ("/p/lib/index.ts", ""),
        ("/p/lib/util.js", ""),
        ("/p/src/index.ts", ""), // alias "~" -> /p/src
        ("/p/src/util.ts", ""),  // extension-alias .js -> .ts ; tsconfig "@app/util"
        // pkg with exports (".", subpath, conditions, wildcard)
        ("/p/node_modules/pkg/package.json", PKG_PKG),
        ("/p/node_modules/pkg/index.js", ""),
        ("/p/node_modules/pkg/sub.js", ""),
        ("/p/node_modules/pkg/c.mjs", ""),
        ("/p/node_modules/pkg/feat/x.js", ""),
        // scoped pkg with main field
        ("/p/node_modules/@scope/comp/package.json", PKG_SCOPE),
        ("/p/node_modules/@scope/comp/main.js", ""),
        // imports-field package
        ("/p/node_modules/imp/package.json", PKG_IMP),
        ("/p/node_modules/imp/index.js", ""),
        ("/p/node_modules/imp/internal.js", ""),
        ("/p/node_modules/imp/n.js", ""),
        // browser-field package
        ("/p/node_modules/brow/package.json", PKG_BROW),
        ("/p/node_modules/brow/main.js", ""),
        ("/p/node_modules/brow/browser.js", ""),
        // deep importer (~8 levels) for the node_modules ancestor-walk magnitude scenario
        ("/p/a1/a2/a3/a4/a5/a6/a7/a8/src/index.js", ""),
    ])
}

fn base_options() -> ResolveOptions {
    ResolveOptions {
        extensions: vec![".ts".into(), ".js".into(), ".json".into()],
        condition_names: vec!["node".into(), "import".into(), "require".into()],
        alias: vec![("~".into(), vec![AliasValue::from("/p/src")])],
        extension_alias: vec![(".js".into(), vec![".ts".into(), ".js".into()])],
        alias_fields: vec![vec!["browser".into()]],
        main_fields: vec!["main".into()],
        builtin_modules: true,
        ..ResolveOptions::default()
    }
}

#[derive(Clone, Copy)]
struct Scenario {
    feature: &'static str,
    importer: &'static str,
    specifier: &'static str,
    /// Resolve with `fully_specified: true` (skips the extension-trial loop).
    fully_specified: bool,
    /// Load `/p/tsconfig.json` and apply it (lights up `read_to_string` + tsconfig `paths`).
    with_tsconfig: bool,
    /// Prime the cache then measure only the second resolve (expected ~0 fs traffic).
    warm: bool,
}

const fn s(feature: &'static str, importer: &'static str, specifier: &'static str) -> Scenario {
    Scenario {
        feature,
        importer,
        specifier,
        fully_specified: false,
        with_tsconfig: false,
        warm: false,
    }
}

fn scenarios() -> Vec<Scenario> {
    vec![
        s("relative", "/p", "./index"),
        s("relative-nested", "/p", "./lib/util"),
        s("relative-parent", "/p/lib", "../index"),
        s("absolute", "/p", "/p/a.js"),
        s("alias", "/p", "~"),
        s("extension-alias", "/p/src", "./util.js"),
        Scenario { with_tsconfig: true, ..s("tsconfig-paths", "/p", "@lib") },
        s("extensions-fallback", "/p", "./foo"),
        s("directory-index", "/p", "./dir"),
        s("exports-dot", "/p", "pkg"),
        s("exports-subpath", "/p", "pkg/sub"),
        s("exports-conditions", "/p", "pkg/cond"),
        s("exports-wildcard", "/p", "pkg/features/x"),
        s("scoped-main", "/p", "@scope/comp"),
        s("imports-field", "/p/node_modules/imp", "#internal"),
        s("imports-conditions", "/p/node_modules/imp", "#cond"),
        s("browser-field", "/p", "brow"),
        s("deep-nm-walk", "/p/a1/a2/a3/a4/a5/a6/a7/a8/src", "pkg"),
        s("query-fragment", "/p", "./index?q=1#frag"),
        s("builtin", "/p", "node:fs"),
        s("file-url", "/p", "file:///p/index.js"),
        s("not-found", "/p", "does-not-exist"),
        Scenario { fully_specified: true, ..s("fully-specified", "/p", "./index.js") },
        Scenario { warm: true, ..s("cache-warm-delta", "/p", "pkg") },
    ]
}

struct Row {
    feature: &'static str,
    ok: bool,
    counts: Snapshot,
    file_deps: usize,
    missing_deps: usize,
}

fn measure(scenario: &Scenario) -> Row {
    let counts = Arc::new(FsCounts::default());
    let fs = CountingFs::wrap(build_fs(), Arc::clone(&counts));

    // tsconfig `paths` are applied by `resolve()` (which auto-discovers `TsconfigDiscovery::Manual`
    // from the options); `resolve_with_context` does not. So this one scenario uses `resolve()` with
    // a Manual tsconfig — it reads the config (counted under `read_to_string`) and applies `paths`.
    // It therefore has no `ResolveContext`, so its dep columns are reported as 0.
    if scenario.with_tsconfig {
        let options = ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: PathBuf::from("/p/tsconfig.json"),
                references: TsconfigReferences::Disabled,
            })),
            ..base_options()
        };
        let resolver = ResolverGeneric::new_with_file_system(fs, options);
        let result = resolver.resolve(scenario.importer, scenario.specifier);
        return Row {
            feature: scenario.feature,
            ok: result.is_ok(),
            counts: counts.snapshot(),
            file_deps: 0,
            missing_deps: 0,
        };
    }

    let options = ResolveOptions { fully_specified: scenario.fully_specified, ..base_options() };
    let resolver = ResolverGeneric::new_with_file_system(fs, options);
    if scenario.warm {
        let _ = resolver.resolve(scenario.importer, scenario.specifier);
        counts.reset();
    }
    let mut ctx = ResolveContext::default();
    let result =
        resolver.resolve_with_context(scenario.importer, scenario.specifier, None, &mut ctx);
    Row {
        feature: scenario.feature,
        ok: result.is_ok(),
        counts: counts.snapshot(),
        file_deps: ctx.file_dependencies.len(),
        missing_deps: ctx.missing_dependencies.len(),
    }
}

fn main() {
    use std::fmt::Write as _;

    // Pin process-wide state that would otherwise perturb the walk on some CI runners.
    unsafe {
        std::env::remove_var("NODE_PATH");
        std::env::remove_var("OXC_RESOLVER_YARN_PNP");
    }

    // Enforce the halfbrown <= 32-key bound the determinism of these counts relies on.
    for (label, json) in [
        ("PKG_ROOT", PKG_ROOT),
        ("TSCONFIG", TSCONFIG),
        ("PKG_PKG", PKG_PKG),
        ("PKG_SCOPE", PKG_SCOPE),
        ("PKG_IMP", PKG_IMP),
        ("PKG_BROW", PKG_BROW),
    ] {
        assert_objects_within_32(label, json);
    }

    let rows: Vec<Row> = scenarios().iter().map(measure).collect();

    let mut out = String::new();
    let _ = writeln!(out, "# oxc-resolver fs-call-count snapshot (logical FileSystem-trait calls)");
    let _ = writeln!(
        out,
        "# fresh resolver per scenario (cache-cold unless noted); regenerate with `just fs-calls`."
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<20} {:>3} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>9} {:>9}",
        "feature",
        "ok",
        "read",
        "r2str",
        "stat",
        "lstat",
        "rlink",
        "canon",
        "file_deps",
        "miss_deps"
    );
    for row in &rows {
        let c = row.counts;
        let _ = writeln!(
            out,
            "{:<20} {:>3} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5} {:>9} {:>9}",
            row.feature,
            if row.ok { "y" } else { "n" },
            c.read,
            c.read_to_string,
            c.metadata,
            c.symlink_metadata,
            c.read_link,
            c.canonicalize,
            row.file_deps,
            row.missing_deps,
        );
    }

    let snapshot_path = concat!(env!("CARGO_MANIFEST_DIR"), "/fs.snap");
    std::fs::write(snapshot_path, &out).expect("failed to write fs.snap");
    print!("{out}");
}
