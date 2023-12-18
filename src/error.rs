use std::{io, path::PathBuf, sync::Arc};
use thiserror::Error;

/// All resolution errors.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ResolveError {
    /// Ignored path
    ///
    /// Derived from ignored path (false value) from browser field in package.json
    /// ```json
    /// {
    ///     "browser": {
    ///         "./module": false
    ///     }
    /// }
    /// ```
    /// See <https://github.com/defunctzombie/package-browser-field-spec#ignore-a-module>
    #[error("Path is ignored")]
    Ignored(PathBuf),

    /// Path not found
    #[error("Path not found {0}")]
    NotFound(PathBuf),

    /// Tsconfig not found
    #[error("Tsconfig not found {0}")]
    TsconfigNotFound(PathBuf),

    #[error("{0}")]
    IOError(IOError),

    /// Node.js builtin modules
    ///
    /// This is an error due to not being a Node.js runtime.
    /// The `alias` option can be used to resolve a builtin module to a polyfill.
    #[error("Builtin module")]
    Builtin(String),

    /// All of the aliased extension are not found
    #[error("All of the aliased extension are not found")]
    ExtensionAlias,

    /// The provided path specifier cannot be parsed
    #[error("{0}")]
    Specifier(SpecifierError),

    /// JSON parse error
    #[error("{0:?}")]
    JSON(JSONError),

    /// Restricted by `ResolveOptions::restrictions`
    #[error("Restriction")]
    Restriction(PathBuf),

    #[error("Invalid module \"{0}\" specifier is not a valid subpath for the \"exports\" resolution of {1}")]
    InvalidModuleSpecifier(String, PathBuf),

    #[error("Invalid \"exports\" target \"{0}\" defined for '{1}' in the package config {2}")]
    InvalidPackageTarget(String, String, PathBuf),

    #[error("Package subpath '{0}' is not defined by \"exports\" in {1}")]
    PackagePathNotExported(String, PathBuf),

    #[error("Invalid package config \"{0}\", \"exports\" cannot contain some keys starting with '.' and some not. The exports object must either be an object of package subpath keys or an object of main entry condition name keys only.")]
    InvalidPackageConfig(PathBuf),

    #[error("Default condition should be last one in \"{0}\"")]
    InvalidPackageConfigDefault(PathBuf),

    #[error("Expecting folder to folder mapping. \"{0}\" should end with \"/\"")]
    InvalidPackageConfigDirectory(PathBuf),

    #[error("Package import not defined")]
    PackageImportNotDefined(String),

    #[error("{0} is unimplemented")]
    Unimplemented(&'static str),

    /// Occurs when alias paths reference each other.
    #[error("Recursion in resolving")]
    Recursion,
}

impl ResolveError {
    pub fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored(_))
    }

    pub(crate) fn from_serde_json_error(path: PathBuf, error: &serde_json::Error) -> Self {
        Self::JSON(JSONError {
            path,
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub enum SpecifierError {
    #[error("[ERR_INVALID_ARG_VALUE]: The specifiers must be a non-empty string. Received ''")]
    Empty,
}

/// JSON error from [serde_json::Error].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JSONError {
    pub path: PathBuf,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Error)]
#[error("{0}")]
pub struct IOError(Arc<io::Error>);

impl PartialEq for IOError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
}

impl From<IOError> for std::io::Error {
    fn from(error: IOError) -> Self {
        let io_error = error.0.as_ref();
        Self::new(io_error.kind(), io_error.to_string())
    }
}

impl From<io::Error> for ResolveError {
    fn from(err: io::Error) -> Self {
        Self::IOError(IOError(Arc::new(err)))
    }
}
