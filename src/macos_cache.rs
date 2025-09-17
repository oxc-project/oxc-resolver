use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::macos::MacOsFs;

/// Package.json cache optimized for macOS
/// Uses F_NOCACHE for one-time reads
pub struct PackageJsonCache {
    cache: papaya::HashMap<PathBuf, Arc<str>, rustc_hash::FxBuildHasher>,
}

impl Default for PackageJsonCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageJsonCache {
    #[must_use]
    pub fn new() -> Self {
        Self { cache: papaya::HashMap::builder().hasher(rustc_hash::FxBuildHasher).build() }
    }

    /// Read package.json with F_NOCACHE on macOS
    ///
    /// # Errors
    ///
    /// * Returns any I/O or UTF-8 validation error produced while reading from disk.
    pub fn read_package_json(&self, path: &Path) -> io::Result<Arc<str>> {
        let pin = self.cache.pin();

        if let Some(cached) = pin.get(path) {
            return Ok(Arc::clone(cached));
        }

        // Use nocache read on macOS to avoid polluting system cache
        let bytes = MacOsFs::read_nocache(path)?;

        // Validate UTF-8
        if simdutf8::basic::from_utf8(&bytes).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "package.json is not valid UTF-8",
            ));
        }

        // SAFETY: the UTF-8 validity is checked above, so the unchecked conversion is sound.
        let content = Arc::from(unsafe { String::from_utf8_unchecked(bytes) });
        pin.insert(path.to_path_buf(), Arc::clone(&content));

        Ok(content)
    }

    pub fn clear(&self) {
        self.cache.pin().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_json_cache() {
        let cache = PackageJsonCache::new();
        let temp_dir = std::env::temp_dir();
        let pkg_path = temp_dir.join("test_package.json");

        // Write test package.json
        std::fs::write(&pkg_path, r#"{"name": "test", "version": "1.0.0"}"#).unwrap();

        // First read should hit filesystem
        let content1 = cache.read_package_json(&pkg_path).unwrap();
        assert!(content1.contains("test"));

        // Second read should hit cache
        let content2 = cache.read_package_json(&pkg_path).unwrap();
        assert_eq!(content1, content2);

        // Clean up
        std::fs::remove_file(&pkg_path).unwrap();
    }
}
