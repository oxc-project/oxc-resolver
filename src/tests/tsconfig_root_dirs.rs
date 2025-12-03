//! Tests for tsconfig root_dirs
//!
//! The rootDirs option allows mapping multiple source directories to a single
//! output directory structure. This is commonly used in build systems where
//! generated code and source code should appear to be in the same directory structure.
//!
//! TypeScript reference: https://www.typescriptlang.org/tsconfig#rootDirs

use crate::{
    ResolveError, ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};

#[test]
fn test() {
    let f = super::fixture_root().join("tsconfig/cases");

    #[rustfmt::skip]
    let pass = [
        // Test Case 1: Initial resolution succeeds
        // Import ./foo from src/index.ts resolves to generated/foo.ts
        (f.join("root-dirs"), "src/index.ts", "./foo", f.join("root-dirs/generated/foo.ts")),

        // Test Case 2: Alternative rootDir succeeds
        // Import ./bar from src/index.ts, bar doesn't exist in src/ but exists in generated/
        (f.join("root-dirs-multiple"), "src/index.ts", "./bar", f.join("root-dirs-multiple/generated/bar.ts")),

        // Test Case 3: Multiple alternatives - second alternative
        // Import ./baz from src/index.ts, baz doesn't exist in src/ or generated/, but exists in lib/
        (f.join("root-dirs-multiple"), "src/index.ts", "./baz", f.join("root-dirs-multiple/lib/baz.ts")),

        // Test Case 4: Longest prefix match
        // Import ./foo from src/sub/index.ts with nested rootDirs ["./src", "./src/sub"]
        // Should match ./src/sub (longer prefix) and resolve to src/sub/foo.ts
        (f.join("root-dirs-nested"), "src/sub/index.ts", "./foo", f.join("root-dirs-nested/src/sub/foo.ts")),

        // Test Case 5: Directory resolution with rootDirs
        // Resolving ./src/index from root directory
        (f.join("root-dirs"), ".", "./src/index", f.join("root-dirs/src/index.ts")),

        // Test Case 6: Parent directory navigation
        // Import ../folder1/file1 from generated/folder2/file3.ts resolves to folder1/file1.ts
        (f.join("root-dirs-parent-nav"), "generated/folder2/file3.ts", "../folder1/file1", f.join("root-dirs-parent-nav/folder1/file1.ts")),

        // Test Case 7: Directory import with parent navigation
        // Import ../folder1/file1_1 from generated/folder2/file3.ts resolves to folder1/file1_1/index.ts
        (f.join("root-dirs-parent-dir"), "generated/folder2/file3.ts", "../folder1/file1_1", f.join("root-dirs-parent-dir/folder1/file1_1/index.ts")),

        // Test Case 8: Single rootDir entry
        // Verify that rootDirs works correctly with only one entry
        (f.join("root-dirs-single"), "src/index.ts", "./foo", f.join("root-dirs-single/src/foo.ts")),

        // Test Case 9: Trailing slash in rootDirs configuration
        // Verify ./src/ works the same as ./src
        (f.join("root-dirs-trailing-slash"), "src/index.ts", "./foo", f.join("root-dirs-trailing-slash/generated/foo.ts")),
    ];

    for (dir, from_path, request, expected) in pass {
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: dir.join("tsconfig.json"),
                references: TsconfigReferences::Auto,
            })),
            extensions: vec![".ts".into()],
            ..ResolveOptions::default()
        });
        let path = dir.join(from_path);
        let resolved_path = resolver.resolve(&path, request).map(|f| f.full_path());
        assert_eq!(resolved_path, Ok(expected), "from {path:?} resolve {request}");
    }

    #[rustfmt::skip]
    let fail = [
        // Test Case 6: No rootDirs configured
        // Without rootDirs, relative imports use normal resolution only
        (f.join("index"), "src/index.ts", "./foo"),

        // Test Case 7: All resolutions fail
        // File doesn't exist in any rootDir
        (f.join("root-dirs-all-fail"), "src/index.ts", "./nonexistent"),

        // Test Case 8: No matching prefix
        // Importing from root when rootDirs are "./src" and "./src/sub"
        // The candidate doesn't match any rootDir prefix
        (f.join("root-dirs-nested"), ".", "./nonexistent"),
    ];

    for (dir, from_path, request) in fail {
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: Some(TsconfigDiscovery::Manual(TsconfigOptions {
                config_file: dir.join("tsconfig.json"),
                references: TsconfigReferences::Auto,
            })),
            extensions: vec![".ts".into()],
            ..ResolveOptions::default()
        });
        let path = dir.join(from_path);
        let resolved_path = resolver.resolve(&path, request);
        assert!(
            matches!(resolved_path, Err(ResolveError::NotFound(_))),
            "expected NotFound error from {path:?} resolve {request}, got {resolved_path:?}"
        );
    }
}
