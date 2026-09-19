use std::{
    hash::BuildHasherDefault,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use dashmap::DashMap;
use rustc_hash::{FxHashMap, FxHasher};

use crate::{PathUtil, ResolveError};

/// Error returned by the path-based fallback in Node's package-map resolution algorithm.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum FindPackageIdError {
    AmbiguousResolution,
    ExternalFile,
}

pub(super) trait PackageMapBackend {
    type Entry<'a>: PackageMapEntryBackend<'a>
    where
        Self: 'a;

    fn package(&self, package_id: &str) -> Option<Self::Entry<'_>>;
    fn iter(&self) -> impl Iterator<Item = (&str, Self::Entry<'_>)>;
}

pub(super) trait PackageMapEntryBackend<'a> {
    fn url(&self) -> &'a str;
    fn dependency(&self, specifier: &str) -> Option<&'a str>;
}

#[derive(Debug, Clone)]
enum PackageOwner {
    Package(Arc<str>),
    Ambiguous,
    External,
}

/// Parsed Node.js package map and its resolved package-location index.
///
/// This represents the specification's top-level `packages` object. See
/// [Configuration file format](https://nodejs.org/api/packages.html#configuration-file-format).
pub(super) struct PackageMapGeneric<S> {
    path: PathBuf,
    store: S,
    package_paths: FxHashMap<Arc<str>, Arc<Path>>,
    path_index: FxHashMap<Arc<Path>, PackageOwner>,
    path_cache: DashMap<PathBuf, PackageOwner, BuildHasherDefault<FxHasher>>,
}

#[cfg(target_endian = "big")]
pub(super) type PackageMap = PackageMapGeneric<super::serde::PackageMapData>;
#[cfg(target_endian = "little")]
pub(super) type PackageMap = PackageMapGeneric<super::simd::PackageMapCell>;

impl<S: PackageMapBackend> PackageMapGeneric<S> {
    pub(super) fn new(path: PathBuf, store: S) -> Result<Self, ResolveError> {
        let mut package_paths = FxHashMap::default();
        let mut path_index = FxHashMap::default();

        for (package_id, entry) in store.iter() {
            let url = entry.url();
            if url.is_empty() {
                return Err(Self::invalid(
                    &path,
                    format!("package {package_id:?} has an empty \"url\" field"),
                ));
            }
            let package_path = Self::resolve_url_from(&path, url).map_err(|reason| {
                Self::invalid(&path, format!("package {package_id:?} has {reason}"))
            })?;
            let package_id = Arc::<str>::from(package_id);
            let package_path = Arc::<Path>::from(package_path);

            package_paths.insert(Arc::clone(&package_id), Arc::clone(&package_path));
            path_index
                .entry(package_path)
                .and_modify(|owner| *owner = PackageOwner::Ambiguous)
                .or_insert(PackageOwner::Package(package_id));
        }

        Ok(Self {
            path,
            store,
            package_paths,
            path_index,
            path_cache: DashMap::with_hasher(BuildHasherDefault::default()),
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn package<'a>(
        &'a self,
        package_id: &str,
    ) -> Option<PackageMapEntryGeneric<'a, S::Entry<'a>>> {
        self.store.package(package_id).map(|entry| PackageMapEntryGeneric {
            entry,
            path: self
                .package_paths
                .get(package_id)
                .map(Arc::as_ref)
                .expect("a parsed package entry must have a resolved path"),
        })
    }

    /// Implements the path-based fallback for determining which package owns an importer.
    ///
    /// The nearest ancestor present in the resolved-path index owns `path`. If multiple package
    /// IDs resolve to that ancestor, this returns [`FindPackageIdError::AmbiguousResolution`]. If
    /// no mapped package contains `path`, it returns [`FindPackageIdError::ExternalFile`].
    ///
    /// This corresponds to `FIND_PACKAGE_ID(PATH, PACKAGE_MAP)` in Node's
    /// [CommonJS resolution pseudocode](https://nodejs.org/api/modules.html#all-together).
    pub(super) fn find_package_id<'a>(
        &'a self,
        path: &Path,
    ) -> Result<&'a str, FindPackageIdError> {
        if let Some(owner) = self.path_cache.get(path) {
            return self.package_id_for_owner(owner.value());
        }

        let normalized_path;
        let lookup_path = if path.components().any(|component| component == Component::ParentDir) {
            normalized_path = path.normalize();
            normalized_path.as_path()
        } else {
            path
        };
        let owner = lookup_path
            .ancestors()
            .find_map(|path| self.path_index.get(path).cloned())
            .unwrap_or(PackageOwner::External);
        let result = self.package_id_for_owner(&owner);
        self.path_cache.insert(path.to_path_buf(), owner);
        result
    }

    fn package_id_for_owner<'a>(
        &'a self,
        owner: &PackageOwner,
    ) -> Result<&'a str, FindPackageIdError> {
        match owner {
            PackageOwner::Package(package_id) => Ok(self
                .package_paths
                .get_key_value(package_id.as_ref())
                .expect("an indexed package ID must have a corresponding resolved path")
                .0
                .as_ref()),
            PackageOwner::Ambiguous => Err(FindPackageIdError::AmbiguousResolution),
            PackageOwner::External => Err(FindPackageIdError::ExternalFile),
        }
    }

    /// Resolves an entry URL against the package-map URL using WHATWG URL semantics.
    fn resolve_url_from(package_map_path: &Path, value: &str) -> Result<PathBuf, String> {
        // WHATWG URL parsing trims leading and trailing C0 controls and spaces, and removes ASCII
        // tabs and newlines anywhere in the input. Backslashes are path separators for `file:`
        // URLs, including relative URLs resolved against a `file:` base.
        let value = value.trim_matches(|character: char| character <= '\u{20}');
        let normalized_value;
        let value = if value.contains(['\\', '\t', '\n', '\r']) {
            normalized_value = value
                .chars()
                .filter_map(|character| match character {
                    '\t' | '\n' | '\r' => None,
                    '\\' => Some('/'),
                    character => Some(character),
                })
                .collect::<String>();
            normalized_value.as_str()
        } else {
            value
        };
        let value = value.split_once(['?', '#']).map_or(value, |(path, _)| path);

        let scheme = Self::split_url_scheme(value);
        if let Some((scheme, _)) = scheme
            && !scheme.eq_ignore_ascii_case("file")
        {
            return Err(format!(
                "an unsupported URL scheme in {value:?}; only file URLs and relative URLs are supported"
            ));
        }
        if Self::has_invalid_percent_encoding(value) {
            return Err(format!("an invalid file URL {value:?}"));
        }

        let relative = if value.starts_with("//") {
            return Self::file_url_to_path(&format!("file:{value}"), value);
        } else if let Some((_, rest)) = scheme {
            if rest.starts_with("//") {
                return Self::file_url_to_path(&format!("file:{rest}"), value);
            }
            if rest.starts_with('/') || Self::starts_with_windows_drive(rest) {
                return Self::file_url_to_path(&format!("file://{rest}"), value);
            }
            rest
        } else {
            value
        };

        if Self::has_encoded_separator(relative) {
            return Err(format!("an invalid file URL {value:?}"));
        }
        let base = package_map_path
            .parent()
            .ok_or_else(|| "an invalid configuration file path".to_string())?;
        if relative.is_empty() {
            return Ok(base.to_path_buf());
        }
        let decoded = percent_encoding::percent_decode_str(relative)
            .decode_utf8()
            .map_err(|_| format!("an invalid file URL {value:?}"))?;
        Ok(base.normalize_with(Path::new(decoded.as_ref())))
    }

    fn file_url_to_path(url: &str, value: &str) -> Result<PathBuf, String> {
        crate::file_url::resolve_file_protocol(url)
            .map(|path| PathBuf::from(path.as_ref()).normalize())
            .map_err(|_| format!("an invalid file URL {value:?}"))
    }

    fn split_url_scheme(value: &str) -> Option<(&str, &str)> {
        let mut bytes = value.bytes();
        if !bytes.next()?.is_ascii_alphabetic() {
            return None;
        }
        for (index, byte) in value.bytes().enumerate().skip(1) {
            if byte == b':' {
                return Some((&value[..index], &value[index + 1..]));
            }
            if !byte.is_ascii_alphanumeric() && !matches!(byte, b'+' | b'-' | b'.') {
                return None;
            }
        }
        None
    }

    fn starts_with_windows_drive(value: &str) -> bool {
        let bytes = value.as_bytes();
        bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
    }

    fn has_invalid_percent_encoding(value: &str) -> bool {
        let bytes = value.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%'
                && (index + 2 >= bytes.len()
                    || !bytes[index + 1].is_ascii_hexdigit()
                    || !bytes[index + 2].is_ascii_hexdigit())
            {
                return true;
            }
            index += if bytes[index] == b'%' { 3 } else { 1 };
        }
        false
    }

    fn has_encoded_separator(value: &str) -> bool {
        value.as_bytes().windows(3).any(|bytes| {
            bytes[0] == b'%'
                && ((bytes[1] == b'2' && bytes[2].eq_ignore_ascii_case(&b'f'))
                    || (cfg!(windows) && bytes[1] == b'5' && bytes[2].eq_ignore_ascii_case(&b'c')))
        })
    }

    fn invalid(package_map_path: &Path, reason: String) -> ResolveError {
        ResolveError::PackageMapInvalid { package_map_path: package_map_path.to_path_buf(), reason }
    }
}

pub(super) struct PackageMapEntryGeneric<'a, E> {
    entry: E,
    path: &'a Path,
}

impl<'a, E: PackageMapEntryBackend<'a>> PackageMapEntryGeneric<'a, E> {
    pub(super) const fn path(&self) -> &'a Path {
        self.path
    }

    pub(super) fn dependency(&self, specifier: &str) -> Option<&'a str> {
        self.entry.dependency(specifier)
    }
}
