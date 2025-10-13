use crate::{ResolveError, ResolveOptions};
use std::path::{Path, PathBuf};

pub struct TypeReferenceResolver<'a> {
    options: &'a ResolveOptions,
}

impl<'a> TypeReferenceResolver<'a> {
    #[must_use]
    pub fn new(options: &'a ResolveOptions) -> Self {
        Self { options }
    }

    #[must_use]
    pub fn get_effective_type_roots(&self, containing_directory: &Path) -> (Vec<PathBuf>, bool) {
        if let Some(ts_options) = &self.options.typescript_options
            && let Some(type_roots) = &ts_options.type_roots
        {
            return (type_roots.clone(), true);
        }

        let base_dir = if let Some(dir) =
            self.options.tsconfig.as_ref().and_then(|t| t.config_file.parent())
        {
            dir
        } else {
            containing_directory
        };

        let mut type_roots = Vec::with_capacity(base_dir.ancestors().count());
        let mut current = base_dir.to_path_buf();

        loop {
            type_roots.push(current.join("node_modules").join("@types"));

            if !current.pop() {
                break;
            }
        }

        (type_roots, false)
    }

    fn get_default_type_roots(&self, containing_directory: &Path) -> Vec<PathBuf> {
        let mut type_roots = Vec::new();

        let mut current = containing_directory.to_path_buf();
        loop {
            let node_modules = current.join("node_modules");
            if node_modules.is_dir() {
                type_roots.push(node_modules.join("@types"));
            }

            if !current.pop() {
                break;
            }
        }

        type_roots
    }

    /// Resolve a type reference from the given type roots.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::NotFound` if the type reference cannot be resolved.
    pub fn resolve_from_type_roots(
        type_reference: &str,
        type_roots: &[PathBuf],
    ) -> Result<PathBuf, ResolveError> {
        for type_root in type_roots {
            let candidate = type_root.join(type_reference);

            if candidate.join("index.d.ts").is_file() {
                return Ok(candidate.join("index.d.ts"));
            }

            if candidate.with_extension("d.ts").is_file() {
                return Ok(candidate.with_extension("d.ts"));
            }

            let package_json = candidate.join("package.json");
            if package_json.is_file() {
                if let Ok(content) = std::fs::read_to_string(&package_json) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(types_field) = json.get("types").and_then(|v| v.as_str()) {
                            let types_path = candidate.join(types_field);
                            if types_path.is_file() {
                                return Ok(types_path);
                            }
                        }
                        if let Some(typings_field) = json.get("typings").and_then(|v| v.as_str()) {
                            let typings_path = candidate.join(typings_field);
                            if typings_path.is_file() {
                                return Ok(typings_path);
                            }
                        }
                    }
                }
            }
        }

        Err(ResolveError::NotFound(type_reference.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_type_roots_default() {
        let options = ResolveOptions::default();
        let resolver = TypeReferenceResolver::new(&options);

        let containing_dir = env::current_dir().unwrap();
        let (type_roots, _) = resolver.get_effective_type_roots(&containing_dir);

        for root in &type_roots {
            assert!(root.to_string_lossy().contains("node_modules"));
            assert!(root.to_string_lossy().contains("@types"));
        }
    }

    #[test]
    fn test_get_type_roots_custom() {
        #[cfg(feature = "typescript")]
        {
            use crate::typescript::TypeScriptOptions;
            use std::path::PathBuf;

            let ts_options =
                TypeScriptOptions::new().with_type_roots(vec![PathBuf::from("/custom/types")]);

            let options = ResolveOptions {
                typescript_options: Some(ts_options),
                ..ResolveOptions::default()
            };

            let resolver = TypeReferenceResolver::new(&options);
            let (type_roots, _) = resolver.get_effective_type_roots(Path::new("/any/path"));

            assert_eq!(type_roots.len(), 1);
            assert_eq!(type_roots[0], PathBuf::from("/custom/types"));
        }
    }
}
