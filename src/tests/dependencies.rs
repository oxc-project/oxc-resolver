//! https://github.com/webpack/enhanced-resolve/blob/main/test/dependencies.test.js

#[cfg(not(target_os = "windows"))] // MemoryFS's path separator is always `/` so the test will not pass in windows.
mod windows {
    use std::{path::PathBuf, sync::Arc};

    use super::super::memory_fs::MemoryFS;
    use crate::{FsCache, ResolveContext, ResolveOptions, ResolverGeneric};

    fn file_system() -> MemoryFS {
        MemoryFS::new(&[
            ("/a/b/node_modules/some-module/index.js", ""),
            ("/a/node_modules/module/package.json", r#"{"main":"entry.js"}"#),
            ("/a/node_modules/module/file.js", r#"{"main":"entry.js"}"#),
            ("/node_modules/other-module/file.js", ""),
        ])
    }

    #[test]
    fn test() {
        let file_system = file_system();

        let resolver = ResolverGeneric::new_with_cache(
            Arc::new(FsCache::new(file_system)),
            ResolveOptions {
                extensions: vec![".json".into(), ".js".into()],
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
                "/node_modules/other-module/file.js",
                // These dependencies are different from enhanced-resolve due to different code path to
                // querying the file system
                vec![
                    // symlink checks
                    "/node_modules/other-module/file.js",
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
                    "/a/node_modules/other-module",
                    "/a/b/node_modules/other-module",
                    "/package.json",
                    "/node_modules/other-module/package.json",
                ],
            ),
        ];

        for (name, context, request, result, file_dependencies, missing_dependencies) in data {
            let mut ctx = ResolveContext::default();
            let path = PathBuf::from(context);
            let resolved_path =
                resolver.resolve_with_context(path, request, &mut ctx).map(|r| r.full_path());
            assert_eq!(resolved_path, Ok(PathBuf::from(result)));
            let file_dependencies = file_dependencies.iter().map(PathBuf::from).collect();
            let missing_dependencies = missing_dependencies.iter().map(PathBuf::from).collect();
            assert_eq!(ctx.file_dependencies, file_dependencies, "{name} file_dependencies");
            assert_eq!(
                ctx.missing_dependencies, missing_dependencies,
                "{name} missing_dependencies"
            );
        }
    }
}
