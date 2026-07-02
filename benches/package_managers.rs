use criterion::{Criterion, criterion_group, criterion_main};
use oxc_resolver::Resolver;
use rayon::prelude::*;

use workload::Combo;

fn run_combo(c: &mut Criterion, combo: Combo) {
    let Some(root) = combo.fixture_root_if_installed() else {
        eprintln!(
            "skip pm/{}: fixture not installed (run `just install-bench-fixtures`)",
            combo.slug()
        );
        return;
    };
    let reqs = workload::requests(&root);
    let opts = combo.resolve_options(&root);

    // Correctness gate: every request must resolve and land inside the expected package.
    let resolver = Resolver::new(opts.clone());
    for req in &reqs {
        let resolution = resolver.resolve(&req.importer, req.specifier).unwrap_or_else(|err| {
            panic!(
                "pm/{}: resolve({}, {:?}) failed: {err}",
                combo.slug(),
                req.importer.display(),
                req.specifier
            )
        });
        let path = resolution.path();
        let normalized = path.to_string_lossy().replace('\\', "/");
        assert!(
            workload::matches(&normalized, req),
            "pm/{}: resolve({}, {:?}) -> {} (expected package {:?} ending with {:?})",
            combo.slug(),
            req.importer.display(),
            req.specifier,
            path.display(),
            req.pkg_dir,
            req.internal_path,
        );
    }

    // One bench per combo, modelling a bundler-style build: one Resolver, the
    // workload's 16 requests fanned out across worker threads. Resolver creation
    // is inside the timed body so the .pnp.cjs parse, store discovery, and
    // initial cache warmup all count.
    c.bench_function(&format!("pm/{}", combo.slug()), |b| {
        b.iter(|| {
            let resolver = Resolver::new(opts.clone());
            reqs.par_iter().for_each(|req| {
                _ = resolver.resolve(&req.importer, req.specifier);
            });
        });
    });
}

/// Dense workload: every `lodash-es` module as a bare `lodash-es/<file>` deep import, resolved
/// serially with a cold resolver per iteration. This is the access shape of a bundler graph
/// traversal — hundreds of distinct probes into the same package directory — and is the regime
/// where the per-directory listing cache replaces per-candidate `lstat`s with one `read_dir`.
fn run_dense_combo(c: &mut Criterion, combo: Combo) {
    let Some(root) = combo.fixture_root_if_installed() else {
        eprintln!(
            "skip pm-dense/{}: fixture not installed (run `just install-bench-fixtures`)",
            combo.slug()
        );
        return;
    };
    let lodash_dir = root.join("node_modules/lodash-es");
    let mut specifiers: Vec<String> = std::fs::read_dir(&lodash_dir)
        .expect("lodash-es fixture directory")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|extension| extension == "js")
                .then(|| format!("lodash-es/{name}"))
        })
        .collect();
    specifiers.sort_unstable();
    let importer = root.join("apps/web/nested/deep/src");
    let opts = combo.resolve_options(&root);

    // Correctness gate: every deep import must resolve into lodash-es.
    let resolver = Resolver::new(opts.clone());
    for specifier in &specifiers {
        let resolution = resolver.resolve(&importer, specifier).unwrap_or_else(|err| {
            panic!("pm-dense/{}: resolve({specifier:?}) failed: {err}", combo.slug())
        });
        let normalized = resolution.path().to_string_lossy().replace('\\', "/");
        assert!(
            normalized.contains("/lodash-es/") || normalized.contains("/lodash-es@"),
            "pm-dense/{}: resolve({specifier:?}) -> {normalized}",
            combo.slug()
        );
    }

    c.bench_function(&format!("pm-dense/{}", combo.slug()), |b| {
        b.iter(|| {
            let resolver = Resolver::new(opts.clone());
            for specifier in &specifiers {
                _ = resolver.resolve(&importer, specifier);
            }
        });
    });
}

fn bench_package_managers(c: &mut Criterion) {
    // Pin rayon's pool to a fixed thread count so the parallel fan-out is deterministic across
    // machines / CI runners; otherwise the pool sizes to the host core count, which varies and
    // makes CodSpeed's instrumented instruction count flaky.
    let _ = rayon::ThreadPoolBuilder::new().num_threads(4).build_global();
    for &combo in Combo::ALL {
        #[cfg(not(feature = "yarn_pnp"))]
        if matches!(combo, Combo::YarnPnp) {
            eprintln!("skip pm/yarn-pnp: rebuild with `--features yarn_pnp` to include it");
            continue;
        }
        run_combo(c, combo);
    }
    for combo in [Combo::NpmFlat, Combo::PnpmIsolated] {
        run_dense_combo(c, combo);
    }
}

criterion_group!(package_managers, bench_package_managers);
criterion_main!(package_managers);

mod workload {
    //! Shared workload definitions for the package-manager benchmarks.
    //!
    //! Every combo installs the same template monorepo (with per-PM config
    //! overlays) into `fixtures/bench-pm/installs/<slug>/`. The same list of
    //! [`Request`]s is replayed against each combo so resolution costs are
    //! directly comparable.

    use std::{env, path::PathBuf};

    use oxc_resolver::ResolveOptions;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Combo {
        NpmFlat,
        PnpmIsolated,
        PnpmHoisted,
        YarnFlat,
        YarnIsolated,
        YarnPnp,
        BunFlat,
        BunIsolated,
    }

    impl Combo {
        pub const ALL: &'static [Self] = &[
            Self::NpmFlat,
            Self::PnpmIsolated,
            Self::PnpmHoisted,
            Self::YarnFlat,
            Self::YarnIsolated,
            Self::YarnPnp,
            Self::BunFlat,
            Self::BunIsolated,
        ];

        pub fn slug(self) -> &'static str {
            match self {
                Self::NpmFlat => "npm-flat",
                Self::PnpmIsolated => "pnpm-isolated",
                Self::PnpmHoisted => "pnpm-hoisted",
                Self::YarnFlat => "yarn-flat",
                Self::YarnIsolated => "yarn-isolated",
                Self::YarnPnp => "yarn-pnp",
                Self::BunFlat => "bun-flat",
                Self::BunIsolated => "bun-isolated",
            }
        }

        pub fn fixture_root(self) -> PathBuf {
            env::current_dir().unwrap().join("fixtures/bench-pm/installs").join(self.slug())
        }

        pub fn fixture_root_if_installed(self) -> Option<PathBuf> {
            let root = self.fixture_root();
            let installed = match self {
                Self::YarnPnp => root.join(".pnp.cjs").is_file(),
                _ => root.join("node_modules").is_dir(),
            };
            installed.then_some(root)
        }

        pub fn resolve_options(self, fixture_root: &std::path::Path) -> ResolveOptions {
            let base = ResolveOptions {
                cwd: Some(fixture_root.to_path_buf()),
                condition_names: vec!["node".into(), "require".into()],
                extensions: vec![".js".into(), ".cjs".into(), ".mjs".into(), ".json".into()],
                ..ResolveOptions::default()
            };
            match self {
                #[cfg(feature = "yarn_pnp")]
                Self::YarnPnp => ResolveOptions { yarn_pnp: true, ..base },
                _ => base,
            }
        }
    }

    pub struct Request {
        pub importer: PathBuf,
        pub specifier: &'static str,
        /// The package's directory name as it appears in `node_modules/` (e.g., `"react"`,
        /// `"@babel/runtime"`, `"packages/utils"` for workspace deps). Used to verify the
        /// resolver picked the right package — the resolved path must contain a segment
        /// matching this name in one of several PM-specific forms.
        pub pkg_dir: &'static str,
        /// Path inside the package that the resolver should land on (e.g., `"/index.js"`,
        /// `"/helpers/extends.js"`). The resolved absolute path must end with this string.
        pub internal_path: &'static str,
    }

    /// Returns whether `resolved_path` correctly resolves `req`. The path is checked for two things:
    /// (a) it contains a segment matching `req.pkg_dir` in one of the layout-specific forms, and
    /// (b) it ends with `req.internal_path`.
    ///
    /// Layout-specific segment forms (the slash before each variant matters — it anchors the match):
    /// - `/<pkg>/` — flat layouts (npm-flat, pnpm-hoisted, yarn-flat, bun-flat) plus the inner
    ///   `node_modules/<pkg>/` directory of every isolated layout (pnpm-isolated, yarn-pnp cache,
    ///   bun-isolated)
    /// - `/<pkg>@` — pnpm/bun virtual store dir names like `react@18.3.1`
    /// - `/<pkg>-<kind>-` where kind ∈ {npm, patch, virtual, workspace} — yarn berry descriptor forms,
    ///   covering `.store/<pkg>-npm-<ver>-<hash>/`, `.store/<pkg>-patch-<hash>/`, and `.yarn/cache/<pkg>-npm-...zip/`
    /// - the same `+`/`-` variants with `/` replaced for scoped packages (pnpm uses `@scope+name@`,
    ///   yarn-isolated uses `@scope-name-npm-`)
    pub fn matches(resolved_path: &str, req: &Request) -> bool {
        const KINDS: &[&str] = &["npm-", "patch-", "virtual-", "workspace-"];

        if !resolved_path.ends_with(req.internal_path) {
            return false;
        }
        let scoped_dashed = req.pkg_dir.replace('/', "-");
        let scoped_plus = req.pkg_dir.replace('/', "+");
        let mut candidates: Vec<String> = vec![
            format!("/{}/", req.pkg_dir),
            format!("/{}@", req.pkg_dir),
            format!("/{scoped_plus}@"),
        ];
        for kind in KINDS {
            candidates.push(format!("/{}-{kind}", req.pkg_dir));
            candidates.push(format!("/{scoped_dashed}-{kind}"));
        }
        candidates.iter().any(|c| resolved_path.contains(c.as_str()))
    }

    pub fn requests(root: &std::path::Path) -> Vec<Request> {
        // The resolver takes the containing directory of the importer, not the file path itself.
        let deep = root.join("apps/web/nested/deep/src");
        let shallow = root.join("apps/web/src");
        vec![
            // Bare unscoped, exports field with conditions
            Request {
                importer: deep.clone(),
                specifier: "react",
                pkg_dir: "react",
                internal_path: "/index.js",
            },
            // Subpath through exports
            Request {
                importer: deep.clone(),
                specifier: "react/jsx-runtime",
                pkg_dir: "react",
                internal_path: "/jsx-runtime.js",
            },
            // Scoped, deep subpath with conditional exports
            Request {
                importer: deep.clone(),
                specifier: "@babel/runtime/helpers/extends",
                pkg_dir: "@babel/runtime",
                internal_path: "/helpers/extends.js",
            },
            // Scoped subpath that resolves to a directory's index.js
            Request {
                importer: deep.clone(),
                specifier: "@babel/runtime/regenerator",
                pkg_dir: "@babel/runtime",
                internal_path: "/regenerator/index.js",
            },
            // Conditional exports default branch (require condition)
            Request {
                importer: deep.clone(),
                specifier: "axios",
                pkg_dir: "axios",
                internal_path: "/dist/node/axios.cjs",
            },
            // Exports field exact-path mapping
            Request {
                importer: deep.clone(),
                specifier: "axios/unsafe/utils.js",
                pkg_dir: "axios",
                internal_path: "/lib/utils.js",
            },
            // No exports, main field, ESM-only large package
            Request {
                importer: deep.clone(),
                specifier: "lodash-es",
                pkg_dir: "lodash-es",
                internal_path: "/lodash.js",
            },
            // Direct file inside a large dist directory
            Request {
                importer: deep.clone(),
                specifier: "lodash-es/debounce.js",
                pkg_dir: "lodash-es",
                internal_path: "/debounce.js",
            },
            // Exports field as plain string
            Request {
                importer: deep.clone(),
                specifier: "chalk",
                pkg_dir: "chalk",
                internal_path: "/source/index.js",
            },
            // Large single package, main field only
            Request {
                importer: deep.clone(),
                specifier: "typescript",
                pkg_dir: "typescript",
                internal_path: "/lib/typescript.js",
            },
            // Local workspace package (symlinked across PMs; PnP resolves directly to the workspace dir)
            Request {
                importer: deep.clone(),
                specifier: "@bench/utils",
                pkg_dir: "packages/utils",
                internal_path: "/src/index.js",
            },
            // Workspace package subpath through exports
            Request {
                importer: deep.clone(),
                specifier: "@bench/utils/hash",
                pkg_dir: "packages/utils",
                internal_path: "/src/hash.js",
            },
            // Another workspace package
            Request {
                importer: deep,
                specifier: "@bench/ui",
                pkg_dir: "packages/ui",
                internal_path: "/src/index.js",
            },
            // Shallow importer: react, exercises a shorter ancestor walk
            Request {
                importer: shallow.clone(),
                specifier: "react",
                pkg_dir: "react",
                internal_path: "/index.js",
            },
            // Shallow importer: workspace dep
            Request {
                importer: shallow.clone(),
                specifier: "@bench/utils",
                pkg_dir: "packages/utils",
                internal_path: "/src/index.js",
            },
            // Relative path traversal across the monorepo (from apps/web/src/)
            Request {
                importer: shallow,
                specifier: "../../../packages/utils",
                pkg_dir: "packages/utils",
                internal_path: "/src/index.js",
            },
        ]
    }
}
