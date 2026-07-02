//! package.json definitions
//!
//! The bulk of the `package.json` accessor logic lives here, written once against the
//! [`JsonValue`]/[`JsonObject`] backend traits. Each platform provides a thin backend:
//! on little-endian systems [`simd`] parses with simd-json (zero-copy `BorrowedValue`),
//! on big-endian systems [`serde`] falls back to `serde_json::Value`.

#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

#[cfg(target_endian = "big")]
pub use serde::*;
#[cfg(target_endian = "little")]
pub use simd::*;

use std::{
    ffi::OsStr,
    fmt,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use compact_str::CompactString;
use rustc_hash::{FxHashMap, FxHasher};

use crate::{JSONError, ResolveError, path::PathUtil};

/// Hash of a [`crate::ResolveOptions::alias_fields`] configuration, used to validate that a
/// lazily built [`BrowserIndex`] belongs to the querying resolver's configuration (resolvers
/// created via `clone_with_options` share cached `package.json`s across differing options).
pub fn hash_alias_fields(alias_fields: &[Vec<String>]) -> u64 {
    let mut hasher = FxHasher::default();
    alias_fields.len().hash(&mut hasher);
    for field_path in alias_fields {
        field_path.len().hash(&mut hasher);
        for segment in field_path {
            segment.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Lazily built reverse index over the `"browser"` field(s), see
/// [`PackageJsonGeneric::resolve_browser_field`]. `None` when the configuration has no
/// browser field objects in this `package.json`.
pub struct BrowserIndexSlot {
    alias_fields_hash: u64,
    index: Option<Box<BrowserIndex>>,
}

struct BrowserIndex {
    /// Key file name → entries whose key ends in that file name, in declaration order.
    /// `normalize_with` keeps a key's last `Normal` component, so only these keys can map to
    /// a candidate path with the same file name.
    by_file_name: FxHashMap<Box<OsStr>, Vec<BrowserEntry>>,
    /// Keys without a file name (`.`, `..`, `./`) can map to any path and are always confirmed.
    unindexable: Vec<BrowserEntry>,
}

struct BrowserEntry {
    /// Index into the `browser_fields(alias_fields)` sequence the key came from.
    field: usize,
    /// Position within that field's object, for declaration-order matching.
    pos: usize,
    key: CompactString,
}

/// Check if JSON content is empty or contains only whitespace
fn check_if_empty(json_bytes: &[u8], path: &Path) -> Result<(), JSONError> {
    // Check if content is empty or whitespace-only
    if json_bytes.iter().all(|&b| b.is_ascii_whitespace()) {
        return Err(JSONError {
            path: path.to_path_buf(),
            message: "File is empty".to_string(),
            line: 0,
            column: 0,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageType {
    CommonJs,
    Module,
}

impl PackageType {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        match s {
            "commonjs" => Some(Self::CommonJs),
            "module" => Some(Self::Module),
            _ => None,
        }
    }
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommonJs => f.write_str("commonjs"),
            Self::Module => f.write_str("module"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportsExportsKind {
    String,
    Array,
    Map,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SideEffects<'a> {
    Bool(bool),
    String(&'a str),
    Array(Vec<&'a str>),
}

// ---------------------------------------------------------------------------
// JSON backend abstraction
// ---------------------------------------------------------------------------

/// A JSON value, abstracting over `simd_json::BorrowedValue` and `serde_json::Value`.
///
/// Only the operations the resolver needs are exposed; the variant differences between the
/// two backends (e.g. `BorrowedValue::Static(Bool)` vs `Value::Bool`) are hidden behind
/// [`Self::as_bool`]/[`Self::entry_kind`] so the accessor logic can be written once.
pub trait JsonValue: Sized {
    type Object: JsonObject<Value = Self>;

    fn as_str(&self) -> Option<&str>;
    fn as_bool(&self) -> Option<bool>;
    fn as_slice(&self) -> Option<&[Self]>;
    fn as_object(&self) -> Option<&Self::Object>;
    fn entry_kind(&self) -> ImportsExportsKind;
}

/// A JSON object (string-keyed map), abstracting over the two backends' object types.
pub trait JsonObject {
    type Value: JsonValue<Object = Self>;

    fn get(&self, key: &str) -> Option<&Self::Value>;
    fn iter(&self) -> impl Iterator<Item = (&str, &Self::Value)>;
}

/// Storage holding the parsed root value. The simd backend stores a self-referential cell
/// (the `BorrowedValue` borrows the file bytes); the serde backend stores an owned `Value`.
pub trait PackageJsonBackend {
    type Value<'a>: JsonValue
    where
        Self: 'a;

    fn root(&self) -> &Self::Value<'_>;
}

/// Navigate `fields` along `path` (e.g. `["exports"]` or `["a", "b"]`), returning the value.
fn get_value_by_path<'a, O: JsonObject>(fields: &'a O, path: &[String]) -> Option<&'a O::Value> {
    let mut value = fields.get(path.first()?.as_str())?;
    for key in &path[1..] {
        value = value.as_object()?.get(key.as_str())?;
    }
    Some(value)
}

/// Interpret a `"browser"`/alias-field value: a string is the replacement, `false` means the
/// request is ignored, anything else is "no mapping".
fn alias_value<'a, V: JsonValue>(
    key: &Path,
    value: &'a V,
) -> Result<Option<&'a str>, ResolveError> {
    if let Some(s) = value.as_str() {
        return Ok(Some(s));
    }
    if value.as_bool() == Some(false) {
        return Err(ResolveError::Ignored(key.to_path_buf()));
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// PackageJson (generic over the backend)
// ---------------------------------------------------------------------------

/// Generic `package.json`. The public, per-target `PackageJson` is a type alias over this
/// (see the backend modules), so the rest of the crate is unaware of the generic parameter.
pub struct PackageJsonGeneric<S> {
    /// Path to `package.json`. Contains the `package.json` filename.
    pub path: PathBuf,

    /// Realpath to `package.json`. Contains the `package.json` filename.
    pub realpath: PathBuf,

    pub(crate) store: S,

    /// Reverse index for the `"browser"` field, built on first reverse lookup.
    pub(crate) browser_index: OnceLock<BrowserIndexSlot>,
}

impl<S: PackageJsonBackend> fmt::Debug for PackageJsonGeneric<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageJson")
            .field("path", &self.path)
            .field("realpath", &self.realpath)
            .field("name", &self.name())
            .field("type", &self.r#type())
            .finish_non_exhaustive()
    }
}

impl<S: PackageJsonBackend> PackageJsonGeneric<S> {
    /// Returns the path where the `package.json` was found.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This does not need to be the path where the file is stored on disk.
    /// See [Self::realpath()].
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the path where the `package.json` file was stored on disk.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This is the canonicalized version of [Self::path()], where all symbolic
    /// links are resolved.
    #[must_use]
    pub fn realpath(&self) -> &Path {
        &self.realpath
    }

    /// Directory to `package.json`.
    ///
    /// # Panics
    ///
    /// * When the `package.json` path is misconfigured.
    #[must_use]
    pub fn directory(&self) -> &Path {
        debug_assert!(self.realpath.file_name().is_some_and(|x| x == "package.json"));
        self.realpath.parent().unwrap()
    }

    fn field(&self, key: &str) -> Option<&S::Value<'_>> {
        self.store.root().as_object()?.get(key)
    }

    /// Name of the package.
    ///
    /// The "name" field can be used together with the "exports" field to
    /// self-reference a package using its name.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.field("name")?.as_str()
    }

    /// Version of the package.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.field("version")?.as_str()
    }

    /// Returns the package type, if one is configured in the `package.json`.
    ///
    /// <https://nodejs.org/api/packages.html#type>
    #[must_use]
    pub fn r#type(&self) -> Option<PackageType> {
        PackageType::from_str(self.field("type")?.as_str()?)
    }

    /// The "sideEffects" field.
    ///
    /// <https://webpack.js.org/guides/tree-shaking>
    #[must_use]
    pub fn side_effects(&self) -> Option<SideEffects<'_>> {
        let value = self.field("sideEffects")?;
        if let Some(b) = value.as_bool() {
            return Some(SideEffects::Bool(b));
        }
        if let Some(s) = value.as_str() {
            return Some(SideEffects::String(s));
        }
        value
            .as_slice()
            .map(|arr| SideEffects::Array(arr.iter().filter_map(JsonValue::as_str).collect()))
    }

    /// The "exports" field allows defining the entry points of a package.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    #[must_use]
    pub fn exports(&self) -> Option<ImportsExportsEntryGeneric<'_, S::Value<'_>>> {
        Some(ImportsExportsEntryGeneric(self.field("exports")?))
    }

    /// The "types" field in package.json.
    ///
    /// Used by TypeScript to find type declarations for a package.
    #[must_use]
    pub fn types(&self) -> Option<&str> {
        self.field("types")?.as_str()
    }

    /// The "typings" field in package.json (legacy equivalent of "types").
    ///
    /// Used by TypeScript to find type declarations for a package.
    #[must_use]
    pub fn typings(&self) -> Option<&str> {
        self.field("typings")?.as_str()
    }

    /// The "typesVersions" field in package.json.
    ///
    /// Returns the raw JSON value for the "typesVersions" field, which maps
    /// TypeScript version ranges to path redirect maps.
    ///
    /// <https://www.typescriptlang.org/docs/handbook/declaration-files/publishing.html#version-selection-with-typesversions>
    pub(crate) fn types_versions(
        &self,
    ) -> Option<ImportsExportsMapGeneric<'_, <S::Value<'_> as JsonValue>::Object>> {
        Some(ImportsExportsMapGeneric(self.field("typesVersions")?.as_object()?))
    }

    /// The "main" field defines the entry point of a package when imported by
    /// name via a node_modules lookup. Its value should be a path.
    ///
    /// When a package has an "exports" field, this will take precedence over
    /// the "main" field when importing the package by name.
    ///
    /// Values are dynamically retrieved from [crate::ResolveOptions::main_fields].
    ///
    /// <https://nodejs.org/api/packages.html#main>
    pub(crate) fn main_fields<'a>(
        &'a self,
        main_fields: &'a [String],
    ) -> impl Iterator<Item = &'a str> + 'a {
        let object = self.store.root().as_object();
        main_fields
            .iter()
            .filter_map(move |main_field| object?.get(main_field.as_str()))
            .filter_map(JsonValue::as_str)
    }

    /// The "exports" field allows defining the entry points of a package when
    /// imported by name loaded either via a node_modules lookup or a
    /// self-reference to its own name.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    pub(crate) fn exports_fields<'a>(
        &'a self,
        exports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsEntryGeneric<'a, S::Value<'a>>> + 'a {
        let object = self.store.root().as_object();
        exports_fields
            .iter()
            .filter_map(move |object_path| get_value_by_path(object?, object_path))
            .map(ImportsExportsEntryGeneric)
    }

    /// In addition to the "exports" field, there is a package "imports" field
    /// to create private mappings that only apply to import specifiers from
    /// within the package itself.
    ///
    /// <https://nodejs.org/api/packages.html#subpath-imports>
    pub(crate) fn imports_fields<'a>(
        &'a self,
        imports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = ImportsExportsMapGeneric<'a, <S::Value<'a> as JsonValue>::Object>> + 'a
    {
        let object = self.store.root().as_object();
        imports_fields
            .iter()
            .filter_map(move |object_path| get_value_by_path(object?, object_path))
            .filter_map(JsonValue::as_object)
            .map(ImportsExportsMapGeneric)
    }

    fn browser_fields<'a>(
        &'a self,
        alias_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = &'a <S::Value<'a> as JsonValue>::Object> + 'a {
        let object = self.store.root().as_object();
        alias_fields
            .iter()
            .filter_map(move |object_path| get_value_by_path(object?, object_path))
            // Only object is valid, all other types are invalid
            // https://github.com/webpack/enhanced-resolve/blob/3a28f47788de794d9da4d1702a3a583d8422cd48/lib/AliasFieldPlugin.js#L44-L52
            .filter_map(JsonValue::as_object)
    }

    /// Apply this `package.json`'s `"browser"` field (and any other [`crate::ResolveOptions`]
    /// `alias_fields`) to a request or a resolved path.
    ///
    /// * **Forward** (`request` is `Some`): look the request up as a key, remapping it before
    ///   it is resolved on disk (e.g. `module-a` -> `./browser/module-a.js`).
    /// * **Reverse** (`request` is `None`): find the key whose package-relative path equals
    ///   the already-resolved `path`, remapping a file after it is found.
    ///
    /// # Errors
    ///
    /// * [`ResolveError::Ignored`] when the matched value is `false` (request excluded).
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    pub(crate) fn resolve_browser_field<'a>(
        &'a self,
        path: &Path,
        request: Option<&str>,
        alias_fields: &'a [Vec<String>],
        alias_fields_hash: u64,
    ) -> Result<Option<&'a str>, ResolveError> {
        if let Some(request) = request {
            for object in self.browser_fields(alias_fields) {
                // Find matching key in object
                if let Some(value) = object.get(request) {
                    return alias_value(path, value);
                }
            }
            return Ok(None);
        }
        // The reverse lookup runs for every file candidate when `alias_fields` is set, so it
        // goes through a lazily built `file name -> keys` index instead of scanning every key
        // with a `Path::new(key).file_name()` parse each.
        let slot = self.browser_index.get_or_init(|| BrowserIndexSlot {
            alias_fields_hash,
            index: self.build_browser_index(alias_fields),
        });
        if slot.alias_fields_hash == alias_fields_hash {
            let Some(index) = &slot.index else { return Ok(None) };
            return self.browser_reverse_lookup(index, path, alias_fields);
        }
        // The index was built for a resolver with different `alias_fields` sharing this cached
        // `package.json` (`clone_with_options`): fall back to the linear scan.
        let dir = self.path.parent().unwrap();
        let path_file_name = path.file_name();
        for object in self.browser_fields(alias_fields) {
            for (key, value) in object.iter() {
                // Fast path: `normalize_with` keeps `key`'s last component, so a key whose
                // file name differs from the candidate's can't match — skip without
                // allocating. `.`/`..` keys have no `file_name` and fall through.
                if let Some(key_file_name) = Path::new(key).file_name()
                    && Some(key_file_name) != path_file_name
                {
                    continue;
                }
                let joined = dir.normalize_with(key);
                if joined == path {
                    return alias_value(path, value);
                }
            }
        }
        Ok(None)
    }

    fn build_browser_index(&self, alias_fields: &[Vec<String>]) -> Option<Box<BrowserIndex>> {
        let mut by_file_name: FxHashMap<Box<OsStr>, Vec<BrowserEntry>> = FxHashMap::default();
        let mut unindexable = Vec::new();
        let mut has_fields = false;
        for (field, object) in self.browser_fields(alias_fields).enumerate() {
            has_fields = true;
            for (pos, (key, _)) in object.iter().enumerate() {
                let entry = BrowserEntry { field, pos, key: CompactString::new(key) };
                match Path::new(key).file_name() {
                    Some(name) => by_file_name.entry(Box::from(name)).or_default().push(entry),
                    None => unindexable.push(entry),
                }
            }
        }
        has_fields.then(|| Box::new(BrowserIndex { by_file_name, unindexable }))
    }

    /// Reverse lookup through the index: only keys sharing the candidate's file name (plus the
    /// rare file-name-less keys) are confirmed with `normalize_with` + equality, in the same
    /// declaration order (field order, then object order) as the linear scan — first confirmed
    /// key wins.
    fn browser_reverse_lookup<'a>(
        &'a self,
        index: &BrowserIndex,
        path: &Path,
        alias_fields: &'a [Vec<String>],
    ) -> Result<Option<&'a str>, ResolveError> {
        let indexed = path
            .file_name()
            .and_then(|name| index.by_file_name.get(name))
            .map_or(&[][..], Vec::as_slice);
        if indexed.is_empty() && index.unindexable.is_empty() {
            return Ok(None);
        }
        let dir = self.path.parent().unwrap();
        let mut fields = self.browser_fields(alias_fields).enumerate();
        let mut field = fields.next();
        // Merge the two position-sorted entry lists in ascending (field, pos) order.
        let (mut i, mut j) = (0, 0);
        loop {
            let entry = match (indexed.get(i), index.unindexable.get(j)) {
                (Some(a), Some(b)) => {
                    if (a.field, a.pos) < (b.field, b.pos) {
                        i += 1;
                        a
                    } else {
                        j += 1;
                        b
                    }
                }
                (Some(a), None) => {
                    i += 1;
                    a
                }
                (None, Some(b)) => {
                    j += 1;
                    b
                }
                (None, None) => break,
            };
            // `browser_fields` enumerates densely and the index was built from the same
            // sequence, so advancing to `entry.field` always lands on it.
            while field.is_some_and(|(f, _)| f < entry.field) {
                field = fields.next();
            }
            let Some((f, object)) = field else { break };
            if f != entry.field {
                continue;
            }
            let joined = dir.normalize_with(entry.key.as_str());
            if joined == path
                && let Some(value) = object.get(entry.key.as_str())
            {
                return alias_value(path, value);
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// imports/exports field views (generic over the backend)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ImportsExportsEntryGeneric<'a, V>(pub(crate) &'a V);

impl<'a, V: JsonValue> ImportsExportsEntryGeneric<'a, V> {
    #[must_use]
    pub fn kind(&self) -> ImportsExportsKind {
        self.0.entry_kind()
    }

    #[must_use]
    pub fn as_string(&self) -> Option<&'a str> {
        self.0.as_str()
    }

    #[must_use]
    pub fn as_array(&self) -> Option<ImportsExportsArrayGeneric<'a, V>> {
        self.0.as_slice().map(ImportsExportsArrayGeneric)
    }

    #[must_use]
    pub fn as_map(&self) -> Option<ImportsExportsMapGeneric<'a, V::Object>> {
        self.0.as_object().map(ImportsExportsMapGeneric)
    }
}

#[derive(Clone)]
pub struct ImportsExportsArrayGeneric<'a, V>(&'a [V]);

impl<'a, V: JsonValue> ImportsExportsArrayGeneric<'a, V> {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = ImportsExportsEntryGeneric<'a, V>> {
        self.0.iter().map(ImportsExportsEntryGeneric)
    }
}

#[derive(Clone)]
pub struct ImportsExportsMapGeneric<'a, O>(pub(crate) &'a O);

impl<'a, O: JsonObject> ImportsExportsMapGeneric<'a, O> {
    pub fn get(&self, key: &str) -> Option<ImportsExportsEntryGeneric<'a, O::Value>> {
        self.0.get(key).map(ImportsExportsEntryGeneric)
    }

    pub fn keys(&self) -> impl Iterator<Item = &'a str> {
        self.0.iter().map(|(key, _)| key)
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&'a str, ImportsExportsEntryGeneric<'a, O::Value>)> {
        self.0.iter().map(|(key, value)| (key, ImportsExportsEntryGeneric(value)))
    }
}
