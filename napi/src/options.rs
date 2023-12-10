use std::path::PathBuf;

use napi::Either;
use napi_derive::napi;
use std::collections::HashMap;

/// Module Resolution Options
///
/// Options are directly ported from [enhanced-resolve](https://github.com/webpack/enhanced-resolve#resolver-options).
///
/// See [webpack resolve](https://webpack.js.org/configuration/resolve/) for information and examples
#[derive(Debug, Clone)]
#[napi(object)]
pub struct NapiResolveOptions {
    /// Path to TypeScript configuration file.
    ///
    /// Default `None`
    pub tsconfig: Option<TsconfigOptions>,

    /// Alias for [ResolveOptions::alias] and [ResolveOptions::fallback].
    /// For the second value of the tuple, `None -> AliasValue::Ignore`, Some(String) ->
    /// AliasValue::Path(String)`
    /// Create aliases to import or require certain modules more easily.
    /// A trailing $ can also be added to the given object's keys to signify an exact match.
    ///
    pub alias: Option<HashMap<String, Vec<Option<String>>>>,

    /// A list of alias fields in description files.
    /// Specify a field, such as `browser`, to be parsed according to [this specification](https://github.com/defunctzombie/package-browser-field-spec).
    /// Can be a path to json object such as `["path", "to", "exports"]`.
    ///
    /// Default `[]`
    pub alias_fields: Option<Vec<Vec<String>>>,

    /// Condition names for exports field which defines entry points of a package.
    /// The key order in the exports field is significant. During condition matching, earlier entries have higher priority and take precedence over later entries.
    ///
    /// Default `[]`
    pub condition_names: Option<Vec<String>>,

    /// The JSON files to use for descriptions. (There was once a `bower.json`.)
    ///
    /// Default `["package.json"]`
    pub description_files: Option<Vec<String>>,

    /// If true, it will not allow extension-less files.
    /// So by default `require('./foo')` works if `./foo` has a `.js` extension,
    /// but with this enabled only `require('./foo.js')` will work.
    ///
    /// Default to `true` when [ResolveOptions::extensions] contains an empty string.
    /// Use `Some(false)` to disable the behavior.
    /// See <https://github.com/webpack/enhanced-resolve/pull/285>
    ///
    /// Default None, which is the same as `Some(false)` when the above empty rule is not applied.
    pub enforce_extension: Option<EnforceExtension>,

    /// A list of exports fields in description files.
    /// Can be a path to json object such as `["path", "to", "exports"]`.
    ///
    /// Default `[["exports"]]`.
    pub exports_fields: Option<Vec<Vec<String>>>,

    /// An object which maps extension to extension aliases.
    ///
    /// Default `{}`
    pub extension_alias: Option<HashMap<String, Vec<String>>>,

    /// Attempt to resolve these extensions in order.
    /// If multiple files share the same name but have different extensions,
    /// will resolve the one with the extension listed first in the array and skip the rest.
    ///
    /// Default `[".js", ".json", ".node"]`
    pub extensions: Option<Vec<String>>,

    /// Redirect module requests when normal resolving fails.
    ///
    /// Default `[]`
    pub fallback: Option<HashMap<String, Vec<Option<String>>>>,

    /// Request passed to resolve is already fully specified and extensions or main files are not resolved for it (they are still resolved for internal requests).
    ///
    /// See also webpack configuration [resolve.fullySpecified](https://webpack.js.org/configuration/module/#resolvefullyspecified)
    ///
    /// Default `false`
    pub fully_specified: Option<bool>,

    /// A list of main fields in description files
    ///
    /// Default `["main"]`.
    pub main_fields: Option<Vec<String>>,

    /// The filename to be used while resolving directories.
    ///
    /// Default `["index"]`
    pub main_files: Option<Vec<String>>,

    /// A list of directories to resolve modules from, can be absolute path or folder name.
    ///
    /// Default `["node_modules"]`
    pub modules: Option<Vec<String>>,

    /// Resolve to a context instead of a file.
    ///
    /// Default `false`
    pub resolve_to_context: Option<bool>,

    /// Prefer to resolve module requests as relative requests instead of using modules from node_modules directories.
    ///
    /// Default `false`
    pub prefer_relative: Option<bool>,

    /// Prefer to resolve server-relative urls as absolute paths before falling back to resolve in ResolveOptions::roots.
    ///
    /// Default `false`
    pub prefer_absolute: Option<bool>,

    /// A list of resolve restrictions to restrict the paths that a request can be resolved on.
    ///
    /// Default `[]`
    pub restrictions: Option<Vec<Restriction>>,

    /// A list of directories where requests of server-relative URLs (starting with '/') are resolved.
    /// On non-Windows systems these requests are resolved as an absolute path first.
    ///
    /// Default `[]`
    pub roots: Option<Vec<String>>,

    /// Whether to resolve symlinks to their symlinked location.
    /// When enabled, symlinked resources are resolved to their real path, not their symlinked location.
    /// Note that this may cause module resolution to fail when using tools that symlink packages (like npm link).
    ///
    /// Default `true`
    pub symlinks: Option<bool>,

    /// Whether to parse [module.builtinModules](https://nodejs.org/api/module.html#modulebuiltinmodules) or not.
    /// For example, "zlib" will throw [crate::ResolveError::Builtin] when set to true.
    ///
    /// Default `false`
    pub builtin_modules: Option<bool>,
}

#[napi]
#[derive(Debug, PartialEq, Eq)]
pub enum EnforceExtension {
    Auto,
    Enabled,
    Disabled,
}

impl EnforceExtension {
    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }

    pub fn is_enabled(&self) -> bool {
        *self == Self::Enabled
    }

    pub fn is_disabled(&self) -> bool {
        *self == Self::Disabled
    }
}

/// Alias Value for [ResolveOptions::alias] and [ResolveOptions::fallback].
/// Use struct because napi don't support structured union now
#[napi(object)]
#[derive(Debug, Clone)]
pub struct Restriction {
    pub path: Option<String>,
    pub regex: Option<String>,
}

/// Tsconfig Options
///
/// Derived from [tsconfig-paths-webpack-plugin](https://github.com/dividab/tsconfig-paths-webpack-plugin#options)

#[napi(object)]
#[derive(Debug, Clone)]
pub struct TsconfigOptions {
    /// Allows you to specify where to find the TypeScript configuration file.
    /// You may provide
    /// * a relative path to the configuration file. It will be resolved relative to cwd.
    /// * an absolute path to the configuration file.
    pub config_file: String,
    /// Support for Typescript Project References.
    /// For the type TsconfigReferences
    ///
    #[napi(ts_type = "'disabled'| 'auto' | 'Array<string>'")]
    pub references: Either<String, Vec<String>>,
}

// #[derive(Debug, Clone)]
// pub enum TsconfigReferences {
//     Disabled,
//     /// Use the `references` field from tsconfig read from `config_file`.
//     Auto,
//     /// Manually provided relative or absolute path.
//     Paths(Vec<String>),
// }

impl Default for NapiResolveOptions {
    fn default() -> Self {
        Self {
            tsconfig: None,
            alias: Some(HashMap::new()),
            alias_fields: Some(vec![]),
            condition_names: Some(vec![]),
            description_files: Some(vec!["package.json".into()]),
            enforce_extension: Some(EnforceExtension::Auto),
            extension_alias: Some(HashMap::new()),
            exports_fields: Some(vec![vec!["exports".into()]]),
            extensions: Some(vec![".js".into(), ".json".into(), ".node".into()]),
            fallback: Some(HashMap::new()),
            fully_specified: Some(false),
            main_fields: Some(vec!["main".into()]),
            main_files: Some(vec!["index".into()]),
            modules: Some(vec!["node_modules".into()]),
            resolve_to_context: Some(false),
            prefer_relative: Some(false),
            prefer_absolute: Some(false),
            restrictions: Some(vec![]),
            roots: Some(vec![]),
            symlinks: Some(true),
            builtin_modules: Some(false),
        }
    }
}

impl Into<oxc_resolver::Restriction> for Restriction {
    fn into(self) -> oxc_resolver::Restriction {
        match (self.path, self.regex) {
            (None, None) => {
                panic!("Should specifiy path or regex")
            }
            (None, Some(regex)) => oxc_resolver::Restriction::RegExp(regex),
            (Some(path), None) => oxc_resolver::Restriction::Path(PathBuf::from(path)),
            (Some(_), Some(_)) => {
                panic!("Restriction can't be path and regex at the same time")
            }
        }
    }
}

impl Into<oxc_resolver::EnforceExtension> for EnforceExtension {
    fn into(self) -> oxc_resolver::EnforceExtension {
        match self {
            EnforceExtension::Auto => oxc_resolver::EnforceExtension::Auto,
            EnforceExtension::Enabled => oxc_resolver::EnforceExtension::Enabled,
            EnforceExtension::Disabled => oxc_resolver::EnforceExtension::Disabled,
        }
    }
}

impl Into<oxc_resolver::TsconfigOptions> for TsconfigOptions {
    fn into(self) -> oxc_resolver::TsconfigOptions {
        oxc_resolver::TsconfigOptions {
            config_file: PathBuf::from(self.config_file),
            references: match self.references {
                Either::A(string) if string.as_str() == "disabled" => {
                    oxc_resolver::TsconfigReferences::Disabled
                }
                Either::A(string) if string.as_str() == "auto" => {
                    oxc_resolver::TsconfigReferences::Auto
                }
                Either::A(opt) => {
                    panic!("`{}` is not a valid option for  tsconfig references", opt)
                }
                Either::B(paths) => oxc_resolver::TsconfigReferences::Paths(
                    paths.into_iter().map(PathBuf::from).collect::<Vec<_>>(),
                ),
            },
        }
    }
}
