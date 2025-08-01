use std::{
    fmt::{self, Debug, Display},
    io,
    path::PathBuf,
    sync::Arc,
};

use thiserror::Error;

/// All resolution errors
///
/// `thiserror` is used to display meaningful error messages.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
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
    #[error("Path is ignored {0}")]
    Ignored(PathBuf),

    /// Module not found
    #[error("Cannot find module '{0}'")]
    NotFound(/* specifier */ String),

    /// Matched alias value  not found
    #[error("Cannot find module '{0}' for matched aliased key '{1}'")]
    MatchedAliasNotFound(/* specifier */ String, /* alias key */ String),

    /// Tsconfig not found
    #[error("Tsconfig not found {0}")]
    TsconfigNotFound(PathBuf),

    /// Tsconfig's project reference path points to it self
    #[error("Tsconfig's project reference path points to this tsconfig {0}")]
    TsconfigSelfReference(PathBuf),

    /// Occurs when tsconfig extends configs circularly
    #[error("Tsconfig extends configs circularly: {0}")]
    TsconfigCircularExtend(CircularPathBufs),

    #[error("{0}")]
    IOError(IOError),

    /// Indicates the resulting path won't be able consumable by NodeJS `import` or `require`.
    /// For example, DOS device path with Volume GUID (`\\?\Volume{...}`) is not supported.
    #[error("Path {0:?} contains unsupported construct.")]
    PathNotSupported(PathBuf),

    /// Node.js builtin module when `Options::builtin_modules` is enabled.
    ///
    /// `is_runtime_module` can be used to determine whether the request
    /// was prefixed with `node:` or not.
    ///
    /// `resolved` is always prefixed with "node:" in compliance with the ESM specification.
    #[error("Builtin module {resolved}")]
    Builtin { resolved: String, is_runtime_module: bool },

    /// All of the aliased extension are not found
    ///
    /// Displays `Cannot resolve 'index.mjs' with extension aliases 'index.mts' in ...`
    #[error("Cannot resolve '{0}' for extension aliases '{1}' in '{2}'")]
    ExtensionAlias(
        /* File name */ String,
        /* Tried file names */ String,
        /* Path to dir */ PathBuf,
    ),

    /// The provided path specifier cannot be parsed
    #[error("{0}")]
    Specifier(SpecifierError),

    /// JSON parse error
    #[error("{0:?}")]
    Json(JSONError),

    #[error(r#"Invalid module "{0}" specifier is not a valid subpath for the "exports" resolution of {1}"#)]
    InvalidModuleSpecifier(String, PathBuf),

    #[error(r#"Invalid "exports" target "{0}" defined for '{1}' in the package config {2}"#)]
    InvalidPackageTarget(String, String, PathBuf),

    #[error(r#"Package subpath '{0}' is not defined by "exports" in {1}"#)]
    PackagePathNotExported(String, PathBuf),

    #[error(r#"Invalid package config "{0}", "exports" cannot contain some keys starting with '.' and some not. The exports object must either be an object of package subpath keys or an object of main entry condition name keys only."#)]
    InvalidPackageConfig(PathBuf),

    #[error(r#"Default condition should be last one in "{0}""#)]
    InvalidPackageConfigDefault(PathBuf),

    #[error(r#"Expecting folder to folder mapping. "{0}" should end with "/"#)]
    InvalidPackageConfigDirectory(PathBuf),

    #[error(r#"Package import specifier "{0}" is not defined in package {1}"#)]
    PackageImportNotDefined(String, PathBuf),

    #[error("{0} is unimplemented")]
    Unimplemented(&'static str),

    /// Occurs when alias paths reference each other.
    #[error("Recursion in resolving")]
    Recursion,

    #[cfg(feature = "yarn_pnp")]
    #[error("Failed to find yarn pnp manifest in {0}.")]
    FailedToFindYarnPnpManifest(PathBuf),

    #[cfg(feature = "yarn_pnp")]
    #[error("{0}")]
    YarnPnpError(pnp::Error),
}

impl ResolveError {
    #[must_use]
    pub const fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignored(_))
    }

    #[must_use]
    pub fn from_serde_json_error(path: PathBuf, error: &serde_json::Error) -> Self {
        Self::Json(JSONError {
            path,
            message: error.to_string(),
            line: error.line(),
            column: error.column(),
        })
    }
}

/// Error for [ResolveError::Specifier]
#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub enum SpecifierError {
    #[error("The specifiers must be a non-empty string. Received \"{0}\"")]
    Empty(String),
}

/// JSON error from [serde_json::Error]
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

impl From<IOError> for io::Error {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircularPathBufs(Vec<PathBuf>);

impl Display for CircularPathBufs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, path) in self.0.iter().enumerate() {
            if i != 0 {
                write!(f, " -> ")?;
            }
            path.fmt(f)?;
        }
        Ok(())
    }
}

impl From<Vec<PathBuf>> for CircularPathBufs {
    fn from(value: Vec<PathBuf>) -> Self {
        Self(value)
    }
}

#[test]
fn test_into_io_error() {
    use std::io::{self, ErrorKind};
    let error_string = "IOError occurred";
    let string_error = io::Error::new(ErrorKind::Interrupted, error_string.to_string());
    let string_error2 = io::Error::new(ErrorKind::Interrupted, error_string.to_string());
    let resolve_io_error: ResolveError = ResolveError::from(string_error2);

    assert_eq!(resolve_io_error, ResolveError::from(string_error));
    assert_eq!(resolve_io_error.clone(), resolve_io_error);
    let ResolveError::IOError(io_error) = resolve_io_error else { unreachable!() };
    assert_eq!(
        format!("{io_error:?}"),
        r#"IOError(Custom { kind: Interrupted, error: "IOError occurred" })"#
    );
    // fix for https://github.com/web-infra-dev/rspack/issues/4564
    let std_io_error: io::Error = io_error.into();
    assert_eq!(std_io_error.kind(), ErrorKind::Interrupted);
    assert_eq!(std_io_error.to_string(), error_string);
    assert_eq!(
        format!("{std_io_error:?}"),
        r#"Custom { kind: Interrupted, error: "IOError occurred" }"#
    );
}

#[test]
fn test_coverage() {
    let error = ResolveError::NotFound("x".into());
    assert_eq!(format!("{error:?}"), r#"NotFound("x")"#);
    assert_eq!(error.clone(), error);

    let error = ResolveError::Specifier(SpecifierError::Empty("x".into()));
    assert_eq!(format!("{error:?}"), r#"Specifier(Empty("x"))"#);
    assert_eq!(error.clone(), error);
}
