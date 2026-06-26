//! Node.js [package maps](https://nodejs.org/docs/latest/api/packages.html#package-maps).
//!
//! A package map is a single static JSON file (`.package-map.json`) that controls package
//! resolution without relying on the `node_modules` folder structure. It declares a set of
//! packages, each identified by a unique *package id*, with a filesystem location (`url`) and an
//! explicit map of bare-specifier *dependencies* to other package ids.
//!
//! ```json
//! {
//!   "packages": {
//!     "app": {
//!       "url": "./packages/app",
//!       "dependencies": { "@myorg/utils": "utils" }
//!     },
//!     "utils": { "url": "./packages/utils" }
//!   }
//! }
//! ```

use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use serde::Deserialize;

use crate::{ResolveError, path::PathUtil};

/// A parsed `.package-map.json` file.
#[derive(Debug)]
pub struct PackageMap {
    /// Package id -> resolved package entry.
    packages: FxHashMap<String, PackageMapEntry>,
}

/// A single entry in the [`PackageMap`]'s `packages` object.
#[derive(Debug)]
pub struct PackageMapEntry {
    /// Absolute filesystem path decoded from the entry's `url` field.
    path: PathBuf,
    /// Maps a bare-specifier package name to a package id within the same package map.
    dependencies: FxHashMap<String, String>,
}

impl PackageMapEntry {
    /// The resolved package directory.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

/// Raw shape of `.package-map.json` for deserialization.
#[derive(Deserialize)]
struct RawPackageMap {
    #[serde(default)]
    packages: FxHashMap<String, RawPackageMapEntry>,
}

#[derive(Deserialize)]
struct RawPackageMapEntry {
    url: String,
    #[serde(default)]
    dependencies: FxHashMap<String, String>,
}

impl PackageMap {
    /// Parse a `.package-map.json` from its `source`, resolving relative `url`s against the
    /// directory containing `config_path`.
    ///
    /// # Errors
    ///
    /// * [`ResolveError::Json`] when the file is not valid JSON.
    /// * [`ResolveError::PathNotSupported`] when an entry's `url` cannot be turned into a path.
    pub(crate) fn parse(config_path: &Path, source: &str) -> Result<Self, ResolveError> {
        let raw: RawPackageMap = serde_json::from_str(source).map_err(|error| {
            ResolveError::from_serde_json_error(config_path.to_path_buf(), &error)
        })?;
        // Relative `url`s are resolved against the configuration file URL, i.e. its directory.
        let base_dir = config_path.parent().unwrap_or(config_path);
        let mut packages = FxHashMap::default();
        packages.reserve(raw.packages.len());
        for (id, entry) in raw.packages {
            let path = resolve_url(base_dir, &entry.url)?;
            packages.insert(id, PackageMapEntry { path, dependencies: entry.dependencies });
        }
        Ok(Self { packages })
    }

    /// Find the package whose location contains `dir` (the importing file's directory), returning
    /// its package id. The most specific (deepest) package location wins when several match.
    pub(crate) fn importer_package(&self, dir: &Path) -> Option<&str> {
        self.packages
            .iter()
            .filter(|(_, entry)| dir.starts_with(&entry.path))
            .max_by_key(|(_, entry)| entry.path.as_os_str().len())
            .map(|(id, _)| id.as_str())
    }

    /// Resolve a bare-specifier `package_name` declared as a dependency of the package identified
    /// by `importer_id`, returning the target package entry.
    pub(crate) fn resolve_dependency(
        &self,
        importer_id: &str,
        package_name: &str,
    ) -> Option<&PackageMapEntry> {
        let target_id = self.packages.get(importer_id)?.dependencies.get(package_name)?;
        self.packages.get(target_id)
    }
}

/// Resolve an entry `url` to an absolute path.
///
/// Only the `file:` protocol is supported. Non-`file:` URLs are treated as relative references
/// resolved against the configuration file's directory, matching `new URL(url, configFileURL)`.
fn resolve_url(base_dir: &Path, url: &str) -> Result<PathBuf, ResolveError> {
    if url.starts_with("file:") {
        cfg_if::cfg_if! {
            if #[cfg(not(target_arch = "wasm32"))] {
                let path = crate::file_url::resolve_file_protocol(url)?;
                Ok(Path::new(path.as_ref()).normalize())
            } else {
                Err(ResolveError::PathNotSupported(PathBuf::from(url)))
            }
        }
    } else {
        Ok(base_dir.join(url).normalize())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PackageMap;

    #[test]
    fn parses_and_resolves_relative_urls() {
        let config = Path::new("/project/.package-map.json");
        let source = r#"{
            "packages": {
                "app": { "url": "./packages/app", "dependencies": { "utils": "utils" } },
                "utils": { "url": "./packages/utils" }
            }
        }"#;
        let map = PackageMap::parse(config, source).unwrap();

        // Importer lookup picks the deepest matching package, resolving the relative `url`.
        assert_eq!(map.importer_package(Path::new("/project/packages/app/src")), Some("app"));
        assert_eq!(map.importer_package(Path::new("/project/packages/utils")), Some("utils"));
        assert_eq!(map.importer_package(Path::new("/project/outside")), None);

        // Dependency resolution follows the `dependencies` table.
        assert_eq!(
            map.resolve_dependency("app", "utils").unwrap().path(),
            Path::new("/project/packages/utils")
        );
        // `utils` declares no dependencies, and `app` is not one of them.
        assert!(map.resolve_dependency("utils", "app").is_none());
    }

    #[test]
    fn missing_packages_field_is_empty() {
        let map = PackageMap::parse(Path::new("/p/.package-map.json"), "{}").unwrap();
        assert!(map.importer_package(Path::new("/p")).is_none());
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(PackageMap::parse(Path::new("/p/.package-map.json"), "not json").is_err());
    }
}
