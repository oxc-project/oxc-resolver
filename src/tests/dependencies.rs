//! https://github.com/webpack/enhanced-resolve/blob/main/test/dependencies.test.js

#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use rustc_hash::FxHashSet;
    use std::path::PathBuf;

    use crate::{ResolveContext, ResolveOptions, ResolverGeneric};

    use super::super::memory_fs::MemoryFS;

    fn file_system() -> MemoryFS {
        MemoryFS::new(&[
            ("/a/b/node_modules/some-module/index.js", ""),
            ("/a/node_modules/module/package.json", r#"{"main":"entry.js"}"#),
            ("/a/node_modules/module/file.js", r#"{"main":"entry.js"}"#),
            ("/modules/other-module/file.js", ""),
        ])
    }

    #[test]
    fn test() {
        let file_system = file_system();

        let resolver = ResolverGeneric::<MemoryFS>::new_with_file_system(
            file_system,
            ResolveOptions {
                extensions: vec![".json".into(), ".js".into()],
                modules: vec!["/modules".into(), "node_modules".into()],
                ..ResolveOptions::default()
            },
        );

        let data = [
            (
                "middle module request",
                "/a/b/c",
                "module/file",
                "/a/node_modules/module/file.js",
                // These dependencies are different from enhanced-resolve due to different code path to
                // querying the file system
                vec![
                    // found package.json
                    "/a/node_modules/module/package.json",
                    // symlink checks
                    "/a/node_modules/module/file.js",
                    // "/a/node_modules/module",
                    // "/a/node_modules",
                    // "/a",
                    // "/",
                ],
                vec![
                    // missing package.jsons
                    // "/a/b/c/package.json",
                    "/a/b/package.json",
                    "/a/package.json",
                    "/package.json",
                    // missing modules directories
                    "/a/b/c",
                    // "/a/b/c/node_modules",
                    // missing single file modules
                    "/modules/module",
                    "/a/b/node_modules/module",
                    // missing files with alternative extensions
                    "/a/node_modules/module/file",
                    "/a/node_modules/module/file.json",
                ],
            ),
            (
                "fast found module",
                "/a/b/c",
                "other-module/file.js",
                "/modules/other-module/file.js",
                // These dependencies are different from enhanced-resolve due to different code path to
                // querying the file system
                vec![
                    // symlink checks
                    "/modules/other-module/file.js",
                    // "/modules/other-module",
                    // "/modules",
                    // "/",
                ],
                vec![
                    // missing package.jsons
                    // "/a/b/c/package.json",
                    "/a/b/c",
                    "/a/b/package.json",
                    "/a/package.json",
                    "/package.json",
                    "/modules/other-module/package.json",
                    "/modules/package.json",
                ],
            ),
        ];

        for (name, context, request, result, file_dependencies, missing_dependencies) in data {
            let mut ctx = ResolveContext::default();
            let path = PathBuf::from(context);
            let resolved =
                resolver.resolve_with_context(path, request, &mut ctx).map(|r| r.full_path());
            assert_eq!(resolved, Ok(PathBuf::from(result)));
            let file_dependencies =
                FxHashSet::from_iter(file_dependencies.iter().map(PathBuf::from));
            let missing_dependencies =
                FxHashSet::from_iter(missing_dependencies.iter().map(PathBuf::from));
            assert_eq!(ctx.file_dependencies, file_dependencies, "{name}");
            assert_eq!(ctx.missing_dependencies, missing_dependencies, "{name}");
        }
    }
}
