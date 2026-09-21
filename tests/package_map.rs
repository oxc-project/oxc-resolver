#![cfg(not(target_os = "wasi"))]

use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use oxc_resolver::{
    FileMetadata, FileSystem, FileSystemOs, ResolveContext, ResolveError, ResolveOptions, Resolver,
    ResolverGeneric, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

const CHILD_CASE: &str = "OXC_RESOLVER_PACKAGE_MAP_TEST_CASE";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture(path: &str) -> PathBuf {
    root().join("fixtures").join(path)
}

fn escape_node_option(path: &str) -> String {
    path.replace('\\', "\\\\").replace('"', "\\\"")
}

fn node_options(case: &str) -> String {
    let map = match case {
        "resolution" => "./fixtures/package-map/resolution/node_modules/.package-map.json".into(),
        "owners" => fixture("package-map/find-package-id/.package-map.json"),
        "invalid-json" => fixture("package-map/invalid/.package-map.json"),
        "invalid-shape" => fixture("package-map/invalid/invalid-shape.package-map.json"),
        "empty-url" => fixture("package-map/invalid/empty-url.package-map.json"),
        "invalid-dependencies" => {
            fixture("package-map/invalid/invalid-dependencies.package-map.json")
        }
        "invalid-url" => fixture("package-map/invalid/invalid-url.package-map.json"),
        "invalid-authority" => fixture("package-map/invalid/invalid-authority.package-map.json"),
        "invalid-percent" => fixture("package-map/invalid/invalid-percent.package-map.json"),
        "encoded-separator" => fixture("package-map/invalid/encoded-separator.package-map.json"),
        "url-forms" => fixture("package-map/url-forms/.package-map.json"),
        "missing" => fixture("package-map/invalid/missing.package-map.json"),
        "no-map" => return "--trace-warnings".into(),
        "empty" => {
            return "--experimental-package-map=valid.json --experimental-package-map=".into();
        }
        "unterminated" => return "--experimental-package-map=\"unterminated".into(),
        "trailing-escape" => return "--experimental-package-map=\"trailing\\".into(),
        _ => unreachable!(),
    };
    let map = escape_node_option(&map.to_string_lossy());
    match case {
        "resolution" => format!(
            r#"--dummy="escaped\"quote" --experimental-package-map=ignored.json --experimental-package-map="{map}""#
        ),
        "owners" => format!(r#"--experimental-package-map "{map}""#),
        _ => format!(r#"--experimental-package-map="{map}""#),
    }
}

fn resolver(options: ResolveOptions) -> Resolver {
    Resolver::new(options)
}

fn assert_resolution(resolver: &Resolver, importer: &Path, specifier: &str, expected: &Path) {
    assert_eq!(
        resolver.resolve(importer, specifier).map(|resolution| resolution.full_path()),
        Ok(expected.to_path_buf()),
    );
}

fn assert_not_found(resolver: &Resolver, importer: &Path, specifier: &str) {
    assert!(matches!(
        resolver.resolve(importer, specifier),
        Err(ResolveError::NotFound(not_found)) if not_found == specifier
    ));
}

struct CountingPackageMapFs {
    package_map_path: PathBuf,
    package_map_reads: Arc<AtomicUsize>,
}

impl FileSystem for CountingPackageMapFs {
    #[cfg(feature = "yarn_pnp")]
    fn new(_yarn_pnp: bool) -> Self {
        unreachable!()
    }

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        unreachable!()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if path == self.package_map_path {
            self.package_map_reads.fetch_add(1, Ordering::Relaxed);
        }
        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        FileSystemOs::read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        FileSystemOs::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        FileSystemOs::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        FileSystemOs::read_link(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        FileSystemOs::canonicalize(path)
    }
}

fn resolution() {
    let fixture = fixture("package-map/resolution");
    let importer = fixture.join("apps/web/src");
    let options = ResolveOptions {
        condition_names: vec!["node".into(), "require".into()],
        ..ResolveOptions::default()
    };
    let resolver = resolver(options);

    for (base, specifier, expected) in [
        (&importer, "axios", "node_modules/store/axios/index.js"),
        (&importer, "axios/client", "node_modules/store/axios/lib/client.js"),
        (&importer, "@bench/ui", "packages/ui/src/index.js"),
        (&importer, "@bench/web", "apps/web/src/index.js"),
        (&importer, "#react", "node_modules/store/react/index.js"),
        (&importer, "plain-file", "node_modules/store/plain-file.js"),
        (&importer, "plain-directory", "node_modules/store/plain-directory/index.js"),
        (
            &fixture.join("node_modules/store/axios/lib"),
            "follow-redirects",
            "node_modules/store/follow-redirects/index.js",
        ),
        (&fixture.join("node_modules/importer"), "plain-file", "node_modules/store/plain-file.js"),
    ] {
        assert_resolution(&resolver, base, specifier, &fixture.join(expected));
    }

    let unnormalized_importer = fixture.join("packages/ui").join("../..").join("apps/web/src");
    assert_resolution(
        &resolver,
        &unnormalized_importer,
        "axios",
        &fixture.join("node_modules/store/axios/index.js"),
    );
    assert_eq!(
        resolver
            .resolve_file(fixture.join("node_modules/store/plain-file.js"), "react")
            .map(|resolution| resolution.full_path()),
        Ok(fixture.join("node_modules/store/react/index.js")),
    );
    let resolution = resolver
        .resolve_file(fixture.join("node_modules/store/plain-file.js"), "react#fragment")
        .unwrap();
    assert_eq!(resolution.path(), fixture.join("node_modules/store/react/index.js"));
    assert_eq!(resolution.fragment(), Some("#fragment"));

    let context_resolver =
        Resolver::new(ResolveOptions { resolve_to_context: true, ..ResolveOptions::default() });
    assert_resolution(
        &context_resolver,
        &importer,
        "plain-directory",
        &fixture.join("node_modules/store/plain-directory"),
    );

    let browser_resolver = Resolver::new(ResolveOptions {
        alias_fields: vec![vec!["browser".into()]],
        ..ResolveOptions::default()
    });
    assert_resolution(
        &browser_resolver,
        &importer,
        "plain-directory",
        &fixture.join("node_modules/store/react/index.js"),
    );

    assert_not_found(&resolver, &importer, "follow-redirects");
    assert_not_found(&resolver, &importer, "plain-directory/missing");
    assert!(matches!(
        resolver.resolve(&importer, "missing-target"),
        Err(ResolveError::PackageMapKeyNotFound { package_id, .. })
            if package_id == "missing-target"
    ));

    cache(&fixture, &importer);
    tsconfig(&fixture, &importer);
}

fn cache(fixture: &Path, importer: &Path) {
    let package_map_path = fixture.join("node_modules/.package-map.json");
    let package_map_reads = Arc::new(AtomicUsize::new(0));
    let resolver = ResolverGeneric::new_with_file_system(
        CountingPackageMapFs {
            package_map_path,
            package_map_reads: Arc::clone(&package_map_reads),
        },
        ResolveOptions::default(),
    );

    for _ in 0..2 {
        resolver.resolve(importer, "axios").unwrap();
    }
    assert_eq!(package_map_reads.load(Ordering::Relaxed), 1);

    let package_map_node_options = env::var_os("NODE_OPTIONS").unwrap();
    // SAFETY: each package-map case runs as the only test in an isolated child process.
    unsafe { env::set_var("NODE_OPTIONS", "--trace-warnings") };
    resolver.clear_cache();
    assert!(matches!(
        resolver.resolve(importer, "axios"),
        Err(ResolveError::NotFound(specifier)) if specifier == "axios"
    ));
    assert_eq!(package_map_reads.load(Ordering::Relaxed), 1);

    // SAFETY: each package-map case runs as the only test in an isolated child process.
    unsafe { env::set_var("NODE_OPTIONS", package_map_node_options) };
    resolver.resolve(importer, "axios").unwrap_err();
    resolver.clear_cache();
    resolver.resolve(importer, "axios").unwrap();
    assert_eq!(package_map_reads.load(Ordering::Relaxed), 2);
}

fn tsconfig(fixture: &Path, importer: &Path) {
    for config_file in ["tsconfig.package-map.json", "tsconfig.package-map-self.json"] {
        let resolver = resolver(ResolveOptions {
            condition_names: vec!["node".into(), "require".into()],
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: fixture.join("apps/web").join(config_file),
                references: TsconfigReferences::Auto,
            })),
            ..ResolveOptions::default()
        });
        assert_resolution(&resolver, importer, "./index.js", &importer.join("index.js"));
    }
}

fn owners() {
    let fixture = fixture("package-map/find-package-id");
    let package_map_path = fixture.join(".package-map.json");
    let resolver = resolver(ResolveOptions::default());

    for _ in 0..2 {
        let mut context = ResolveContext::default();
        assert!(matches!(
            resolver.resolve_with_context(
                fixture.join("packages/duplicate"),
                "dependency",
                None,
                &mut context,
            ),
            Err(ResolveError::PackageMapAmbiguousResolution { .. })
        ));
        assert!(context.file_dependencies.contains(&package_map_path));

        let mut context = ResolveContext::default();
        assert!(matches!(
            resolver.resolve_with_context(
                fixture.join("external"),
                "dependency",
                None,
                &mut context,
            ),
            Err(ResolveError::PackageMapExternalFile { .. })
        ));
        assert!(context.file_dependencies.contains(&package_map_path));
    }
}

fn invalid(case: &str) {
    let fixture = fixture("package-map/invalid");
    let resolver = resolver(ResolveOptions::default());

    for _ in 0..2 {
        let mut context = ResolveContext::default();
        let error =
            resolver.resolve_with_context(&fixture, "dependency", None, &mut context).unwrap_err();
        if case == "invalid-json" {
            assert!(matches!(error, ResolveError::Json(_)));
        } else {
            let ResolveError::PackageMapInvalid { reason, .. } = error else {
                panic!("expected invalid package map error")
            };
            let expected_reason = match case {
                "empty-url" => Some("empty \"url\" field"),
                "invalid-url" => Some("unsupported URL scheme"),
                "invalid-authority" | "invalid-percent" | "encoded-separator" => {
                    Some("invalid file URL")
                }
                "invalid-shape" | "invalid-dependencies" => None,
                _ => unreachable!(),
            };
            if let Some(expected_reason) = expected_reason {
                assert!(reason.contains(expected_reason), "{reason}");
            }
        }
        assert!(!context.file_dependencies.is_empty());
    }
}

fn no_map() {
    assert_not_found(
        &resolver(ResolveOptions::default()),
        &fixture("package-map/resolution/apps/web/src"),
        "plain-file",
    );
}

fn missing() {
    let fixture = fixture("package-map/invalid");
    let missing_path = fixture.join("missing.package-map.json");
    let mut context = ResolveContext::default();
    resolver(ResolveOptions::default())
        .resolve_with_context(&fixture, "dependency", None, &mut context)
        .unwrap_err();
    assert!(context.missing_dependencies.contains(&missing_path));
}

#[cfg(not(windows))]
fn url_forms() {
    let fixture = fixture("package-map/url-forms");
    assert!(matches!(
        resolver(ResolveOptions::default()).resolve(&fixture, "dependency"),
        Err(ResolveError::PackageMapExternalFile { .. })
    ));
}

fn child(case: &str) {
    match case {
        "resolution" => resolution(),
        "owners" => owners(),
        "invalid-json"
        | "invalid-shape"
        | "empty-url"
        | "invalid-dependencies"
        | "invalid-url"
        | "invalid-authority"
        | "invalid-percent"
        | "encoded-separator" => invalid(case),
        #[cfg(not(windows))]
        "url-forms" => url_forms(),
        "missing" => missing(),
        "no-map" | "empty" | "unterminated" | "trailing-escape" => no_map(),
        _ => unreachable!(),
    }
}

#[test]
fn package_map() {
    if let Ok(case) = env::var(CHILD_CASE) {
        child(&case);
        return;
    }

    for case in [
        "resolution",
        "owners",
        "invalid-json",
        "invalid-shape",
        "empty-url",
        "invalid-dependencies",
        "invalid-url",
        "invalid-authority",
        "invalid-percent",
        "encoded-separator",
        "missing",
        "no-map",
        "empty",
        "unterminated",
        "trailing-escape",
        #[cfg(not(windows))]
        "url-forms",
    ] {
        if cfg!(target_endian = "big") {
            // `cross` runs the test through an emulator, but a child would bypass that runner.
            // SAFETY: this integration test is the only test in its process.
            unsafe { env::set_var("NODE_OPTIONS", node_options(case)) };
            child(case);
            continue;
        }

        let output = Command::new(env::current_exe().unwrap())
            .args(["--exact", "package_map", "--nocapture"])
            .current_dir(root())
            .env(CHILD_CASE, case)
            .env("NODE_OPTIONS", node_options(case))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case} failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}
