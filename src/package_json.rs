use std::{fmt::Display, path::Path};

use crate::ResolveError;

/// Abstract representation for the contents of a `package.json` file, as well
/// as the location where it was found.
///
/// This representation makes no assumptions regarding how the file was
/// deserialized.
#[allow(clippy::missing_errors_doc)] // trait impls should be free to return any typesafe error
pub trait PackageJson: Sized {
    /// Returns the path where the `package.json` was found.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This does not need to be the path where the file is stored on disk.
    /// See [Self::realpath()].
    #[must_use]
    fn path(&self) -> &Path;

    /// Returns the path where the `package.json` file was stored on disk.
    ///
    /// Contains the `package.json` filename.
    ///
    /// This is the canonicalized version of [Self::path()], where all symbolic
    /// links are resolved.
    #[must_use]
    fn realpath(&self) -> &Path;

    /// Directory to `package.json`.
    ///
    /// # Panics
    ///
    /// * When the `package.json` path is misconfigured.
    #[must_use]
    fn directory(&self) -> &Path;

    /// Name of the package.
    ///
    /// The "name" field can be used together with the "exports" field to
    /// self-reference a package using its name.
    ///
    /// <https://nodejs.org/api/packages.html#name>
    fn name(&self) -> Option<&str>;

    /// Returns the package type, if one is configured in the `package.json`.
    ///
    /// <https://nodejs.org/api/packages.html#type>
    fn r#type(&self) -> Option<PackageType>;

    /// The "main" field defines the entry point of a package when imported by
    /// name via a node_modules lookup. Its value should be a path.
    ///
    /// When a package has an "exports" field, this will take precedence over
    /// the "main" field when importing the package by name.
    ///
    /// Values are dynamically retrieved from [crate::ResolveOptions::main_fields].
    ///
    /// <https://nodejs.org/api/packages.html#main>
    #[must_use]
    fn main_fields<'a>(&'a self, main_fields: &'a [String]) -> impl Iterator<Item = &'a str> + 'a;

    /// The "exports" field allows defining the entry points of a package when
    /// imported by name loaded either via a node_modules lookup or a
    /// self-reference to its own name.
    ///
    /// <https://nodejs.org/api/packages.html#exports>
    #[must_use]
    fn exports_fields<'a>(
        &'a self,
        exports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = impl ImportsExportsEntry<'a>> + 'a;

    /// In addition to the "exports" field, there is a package "imports" field
    /// to create private mappings that only apply to import specifiers from
    /// within the package itself.
    ///
    /// <https://nodejs.org/api/packages.html#subpath-imports>
    #[must_use]
    fn imports_fields<'a>(
        &'a self,
        imports_fields: &'a [Vec<String>],
    ) -> impl Iterator<Item = impl ImportsExportsMap<'a>> + 'a;

    /// Resolves the request string for this `package.json` by looking at the
    /// "browser" field.
    ///
    /// <https://github.com/defunctzombie/package-browser-field-spec>
    fn resolve_browser_field<'a>(
        &'a self,
        path: &Path,
        request: Option<&str>,
        alias_fields: &'a [Vec<String>],
    ) -> Result<Option<&'a str>, ResolveError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "fs_cache", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "fs_cache", serde(rename_all = "lowercase"))]
pub enum PackageType {
    CommonJs,
    Module,
}

impl Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommonJs => f.write_str("commonjs"),
            Self::Module => f.write_str("module"),
        }
    }
}

/// Trait used for representing entries in the `imports` and `exports` fields
/// without allocation.
pub trait ImportsExportsEntry<'a>: Clone + Sized {
    type Array: ImportsExportsArray<'a, Entry = Self>;
    type Map: ImportsExportsMap<'a, Entry = Self>;

    fn kind(&self) -> ImportsExportsKind;

    fn as_string(&self) -> Option<&'a str>;
    fn as_array(&self) -> Option<Self::Array>;
    fn as_map(&self) -> Option<Self::Map>;
}

/// Trait used for representing array values in the `imports` and `exports`
/// fields without allocation.
pub trait ImportsExportsArray<'a>: Clone + Sized {
    type Entry: ImportsExportsEntry<'a, Array = Self>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn len(&self) -> usize;
    fn iter(&self) -> impl Iterator<Item = Self::Entry>;
}

/// Trait used for representing map (object) values in the `imports` and
/// `exports` fields without allocation.
pub trait ImportsExportsMap<'a>: Clone + Sized {
    type Entry: ImportsExportsEntry<'a, Map = Self>;

    fn get(&self, key: &str) -> Option<Self::Entry>;
    fn keys(&self) -> impl Iterator<Item = &'a str>;
    fn iter(&self) -> impl Iterator<Item = (&'a str, Self::Entry)>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImportsExportsKind {
    String,
    Array,
    Map,
    Invalid,
}
