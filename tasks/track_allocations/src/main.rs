//! Allocation-count snapshot for oxc-resolver's cache-free parse sub-operations.
//!
//! Mirrors oxc's `tasks/track_memory_allocations`: a counting `#[global_allocator]` over pinned
//! MiMalloc bumps `NUM_ALLOC` / `NUM_REALLOC`, and we record allocation **counts** (not bytes — byte
//! totals vary by platform) for `PackageJson::parse` and `TsConfig::parse` over a spread of fixed
//! inputs, then write a committed `allocs.snap`. CI runs `cargo allocs` and fails the PR on `git diff`.
//!
//! Why these ops: they are **cache-free** on valid input (no `DashMap`, no filesystem walk), so the
//! counts are deterministic and byte-identical between a local machine and CI. Allocation COUNT is a
//! pure function of the input shape (number of keys / nesting), independent of the allocator, SIMD
//! width, core count, or hash seed: parsing pre-sizes containers to the known element count, so the
//! count never depends on hashbrown's seeded bucket placement. This is verified empirically by the
//! intentionally **>32-key** `package_json (exports x40)` fixture, whose count is identical run-to-run
//! and across separate processes (different per-process hash seeds).
//!
//! Whole-`resolve()` allocation tracking is a separate follow-up: it would first need the resolver
//! cache's `DashMap` shard count (`available_parallelism()*4`) pinned, which is a small core change.

use std::{
    alloc::{GlobalAlloc, Layout},
    fmt::Write as _,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering::SeqCst},
};

use mimalloc_safe::MiMalloc;
use oxc_resolver::{FileSystem, FileSystemOs, PackageJson, TsConfig};

static NUM_ALLOC: AtomicUsize = AtomicUsize::new(0);
static NUM_REALLOC: AtomicUsize = AtomicUsize::new(0);

/// A counting allocator that forwards every operation to MiMalloc and tallies allocations.
struct CountingAllocator;

// SAFETY: every method forwards verbatim to `MiMalloc`, which is a sound `GlobalAlloc`; the counters
// are plain atomics touched only on the returned pointer's nullness.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded verbatim to the inner allocator.
        let ptr = unsafe { MiMalloc.alloc(layout) };
        if !ptr.is_null() {
            NUM_ALLOC.fetch_add(1, SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded verbatim to the inner allocator.
        unsafe { MiMalloc.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded verbatim to the inner allocator.
        let ptr = unsafe { MiMalloc.alloc_zeroed(layout) };
        if !ptr.is_null() {
            NUM_ALLOC.fetch_add(1, SeqCst);
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded verbatim to the inner allocator.
        let new_ptr = unsafe { MiMalloc.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            NUM_REALLOC.fetch_add(1, SeqCst);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn reset() {
    NUM_ALLOC.store(0, SeqCst);
    NUM_REALLOC.store(0, SeqCst);
}

fn counts() -> (usize, usize) {
    (NUM_ALLOC.load(SeqCst), NUM_REALLOC.load(SeqCst))
}

/// Measure allocations of `f`. Build all inputs BEFORE calling. Returns (allocs, reallocs).
fn measure<R>(f: impl FnOnce() -> R) -> ((usize, usize), R) {
    reset();
    let r = f();
    let c = counts();
    (c, r)
}

fn new_fs() -> FileSystemOs {
    // `yarn_pnp` is always enabled for this crate (see Cargo.toml), so the constructor takes a bool.
    FileSystemOs::new(false)
}

// ---- small/medium literal fixtures (objects <= 32 keys) ----

const PKG_SMALL: &str = r#"{ "name": "fixture", "version": "1.0.0" }"#;
const PKG_LARGE: &str = r##"{
  "name": "track-alloc-fixture",
  "version": "1.2.3",
  "type": "module",
  "main": "./lib/index.cjs",
  "module": "./lib/index.mjs",
  "types": "./lib/index.d.ts",
  "exports": {
    ".": { "types": "./lib/index.d.ts", "import": "./lib/index.mjs", "require": "./lib/index.cjs" },
    "./feature": { "import": "./lib/feature.mjs", "require": "./lib/feature.cjs" },
    "./package.json": "./package.json"
  },
  "imports": {
    "#internal": "./src/internal.mjs",
    "#env": { "node": "./src/env.node.mjs", "default": "./src/env.mjs" }
  },
  "browser": { "./lib/node.mjs": "./lib/browser.mjs" },
  "sideEffects": false
}"##;
const TSCONFIG_SMALL: &str = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": { "@app/*": ["./src/*"], "@lib": ["./lib/index.ts"] },
    "target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler"
  }
}"#;
const TSCONFIG_OPTIONS: &str = r#"{
  "compilerOptions": {
    "baseUrl": ".", "rootDir": "./src", "outDir": "./dist", "target": "ES2022",
    "module": "ESNext", "moduleResolution": "Bundler", "lib": ["ES2022", "DOM"],
    "strict": true, "esModuleInterop": true, "skipLibCheck": true, "declaration": true,
    "sourceMap": true, "jsx": "react-jsx", "types": ["node"], "resolveJsonModule": true,
    "isolatedModules": true, "noEmit": true, "allowJs": true, "forceConsistentCasingInFileNames": true
  }
}"#;

// ---- generated large fixtures (magnitude) ----

fn gen_pkg_exports(n: usize) -> String {
    let entries: Vec<String> = (0..n).map(|i| format!("\"./s{i}\":\"./s{i}.js\"")).collect();
    format!("{{\"name\":\"x\",\"version\":\"1.0.0\",\"exports\":{{{}}}}}", entries.join(","))
}

fn gen_pkg_imports(n: usize) -> String {
    let entries: Vec<String> = (0..n).map(|i| format!("\"#i{i}\":\"./i{i}.js\"")).collect();
    format!("{{\"name\":\"x\",\"version\":\"1.0.0\",\"imports\":{{{}}}}}", entries.join(","))
}

fn gen_pkg_browser(n: usize) -> String {
    let entries: Vec<String> = (0..n).map(|i| format!("\"./m{i}.js\":\"./b{i}.js\"")).collect();
    format!(
        "{{\"name\":\"x\",\"version\":\"1.0.0\",\"main\":\"./m.js\",\"browser\":{{{}}}}}",
        entries.join(",")
    )
}

fn gen_tsconfig_paths(n: usize) -> String {
    let entries: Vec<String> = (0..n).map(|i| format!("\"@p{i}/*\":[\"./s{i}/*\"]")).collect();
    format!("{{\"compilerOptions\":{{\"baseUrl\":\".\",\"paths\":{{{}}}}}}}", entries.join(","))
}

enum Kind {
    PackageJson,
    Tsconfig,
}

fn main() {
    let fs = new_fs();
    let pkg_path = PathBuf::from("/p/package.json");
    let ts_path = PathBuf::from("/p/tsconfig.json");

    // Build every input BEFORE measuring so input construction is never counted.
    let fixtures: Vec<(&str, Kind, String)> = vec![
        ("package_json (small)", Kind::PackageJson, PKG_SMALL.to_owned()),
        ("package_json (large)", Kind::PackageJson, PKG_LARGE.to_owned()),
        ("package_json (exports x40, >32 keys)", Kind::PackageJson, gen_pkg_exports(40)),
        ("package_json (imports x15)", Kind::PackageJson, gen_pkg_imports(15)),
        ("package_json (browser x15)", Kind::PackageJson, gen_pkg_browser(15)),
        ("tsconfig (small)", Kind::Tsconfig, TSCONFIG_SMALL.to_owned()),
        ("tsconfig (options-heavy)", Kind::Tsconfig, TSCONFIG_OPTIONS.to_owned()),
        ("tsconfig (paths x80)", Kind::Tsconfig, gen_tsconfig_paths(80)),
    ];

    // Warm-up: absorb one-time process initialization (e.g. simd-json runtime CPU detection) so it
    // does not land in the measured counts.
    let _ =
        PackageJson::parse(&fs, pkg_path.clone(), pkg_path.clone(), PKG_LARGE.as_bytes().to_vec());
    let _ = TsConfig::parse(true, &ts_path, &ts_path, TSCONFIG_SMALL.to_owned());

    let mut rows: Vec<(&str, usize, usize)> = Vec::new();
    for (name, kind, json) in &fixtures {
        let (allocs, reallocs, ok) = match kind {
            Kind::PackageJson => {
                let bytes = json.clone().into_bytes(); // built before measure
                let (c, ok) = measure(|| {
                    PackageJson::parse(&fs, pkg_path.clone(), pkg_path.clone(), bytes).is_ok()
                });
                (c.0, c.1, ok)
            }
            Kind::Tsconfig => {
                let input = json.clone(); // built before measure
                let (c, ok) = measure(|| TsConfig::parse(true, &ts_path, &ts_path, input).is_ok());
                (c.0, c.1, ok)
            }
        };
        assert!(ok, "{name} failed to parse");
        rows.push((name, allocs, reallocs));
    }

    let mut out = String::new();
    let _ = writeln!(out, "# oxc-resolver allocation-count snapshot (cache-free parse sub-ops)");
    let _ = writeln!(out, "# allocation COUNTS (not bytes); regenerate with `just allocs`.");
    let _ = writeln!(out);
    let _ = writeln!(out, "{:<40} {:>10} {:>10}", "sub-op", "allocs", "reallocs");
    for (name, allocs, reallocs) in &rows {
        let _ = writeln!(out, "{name:<40} {allocs:>10} {reallocs:>10}");
    }

    let snapshot_path = concat!(env!("CARGO_MANIFEST_DIR"), "/allocs.snap");
    std::fs::write(snapshot_path, &out).expect("failed to write allocs.snap");
    print!("{out}");
}
