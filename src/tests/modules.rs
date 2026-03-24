//! Tests for ResolveOptions.modules (custom module directories)

#[cfg(not(target_os = "windows"))] // MemoryFS path separator is always `/`
mod tests {
    use std::path::PathBuf;

    use super::super::memory_fs::MemoryFS;
    use crate::{ResolveOptions, ResolverGeneric};

    #[test]
    fn custom_module_directory_name() {
        let fs =
            MemoryFS::new(&[("/project/libs/my-lib/index.js", ""), ("/project/src/app.js", "")]);
        let resolver = ResolverGeneric::new_with_file_system(
            fs,
            ResolveOptions { modules: vec!["libs".into()], ..ResolveOptions::default() },
        );
        let result = resolver.resolve("/project/src", "my-lib").map(|r| r.full_path());
        assert_eq!(result, Ok(PathBuf::from("/project/libs/my-lib/index.js")));
    }

    #[test]
    fn absolute_module_directory() {
        let fs =
            MemoryFS::new(&[("/shared/modules/pkg/index.js", ""), ("/project/src/app.js", "")]);
        let resolver = ResolverGeneric::new_with_file_system(
            fs,
            ResolveOptions { modules: vec!["/shared/modules".into()], ..ResolveOptions::default() },
        );
        let result = resolver.resolve("/project/src", "pkg").map(|r| r.full_path());
        assert_eq!(result, Ok(PathBuf::from("/shared/modules/pkg/index.js")));
    }

    #[test]
    fn multiple_module_directories_priority() {
        let fs = MemoryFS::new(&[
            ("/project/custom_modules/pkg/index.js", ""),
            ("/project/node_modules/pkg/index.js", ""),
        ]);
        let resolver = ResolverGeneric::new_with_file_system(
            fs,
            ResolveOptions {
                modules: vec!["custom_modules".into(), "node_modules".into()],
                ..ResolveOptions::default()
            },
        );
        let result = resolver.resolve("/project", "pkg").map(|r| r.full_path());
        assert_eq!(result, Ok(PathBuf::from("/project/custom_modules/pkg/index.js")));
    }

    #[test]
    fn module_directory_fallback() {
        let fs = MemoryFS::new(&[("/project/node_modules/pkg/index.js", "")]);
        let resolver = ResolverGeneric::new_with_file_system(
            fs,
            ResolveOptions {
                modules: vec!["custom_modules".into(), "node_modules".into()],
                ..ResolveOptions::default()
            },
        );
        let result = resolver.resolve("/project", "pkg").map(|r| r.full_path());
        assert_eq!(result, Ok(PathBuf::from("/project/node_modules/pkg/index.js")));
    }

    #[test]
    fn empty_modules_list() {
        let fs = MemoryFS::new(&[("/project/node_modules/pkg/index.js", "")]);
        let resolver = ResolverGeneric::new_with_file_system(
            fs,
            ResolveOptions { modules: vec![], ..ResolveOptions::default() },
        );
        let result = resolver.resolve("/project", "pkg");
        assert!(result.is_err());
    }
}
