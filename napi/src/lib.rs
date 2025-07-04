#[cfg(all(
    feature = "allocator",
    not(any(target_arch = "arm", target_os = "freebsd", target_family = "wasm"))
))]
#[global_allocator]
static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use napi::{Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use oxc_resolver::{ResolveError, ResolveOptions, Resolver};

use self::options::{NapiResolveOptions, StrOrStrList};

mod options;
#[cfg(feature = "tracing-subscriber")]
mod tracing;

#[napi(object)]
pub struct ResolveResult {
    pub path: Option<String>,
    pub error: Option<String>,
    pub builtin: Option<Builtin>,
    /// Module type for this path.
    ///
    /// Enable with `ResolveOptions#moduleType`.
    ///
    /// The module type is computed `ESM_FILE_FORMAT` from the [ESM resolution algorithm specification](https://nodejs.org/docs/latest/api/esm.html#resolution-algorithm-specification).
    ///
    ///  The algorithm uses the file extension or finds the closest `package.json` with the `type` field.
    pub module_type: Option<ModuleType>,

    /// `package.json` path for the given module.
    pub package_json_path: Option<String>,
}

/// Node.js builtin module when `Options::builtin_modules` is enabled.
#[napi(object)]
pub struct Builtin {
    /// Resolved module.
    ///
    /// Always prefixed with "node:" in compliance with the ESM specification.
    pub resolved: String,

    /// Whether the request was prefixed with `node:` or not.
    /// `fs` -> `false`.
    /// `node:fs` returns `true`.
    pub is_runtime_module: bool,
}

fn resolve(resolver: &Resolver, path: &Path, request: &str) -> ResolveResult {
    match resolver.resolve(path, request) {
        Ok(resolution) => ResolveResult {
            path: Some(resolution.full_path().to_string_lossy().to_string()),
            error: None,
            builtin: None,
            module_type: resolution.module_type().map(ModuleType::from),
            package_json_path: resolution
                .package_json()
                .and_then(|p| p.path().to_str())
                .map(|p| p.to_string()),
        },
        Err(err) => {
            let error = err.to_string();
            ResolveResult {
                path: None,
                builtin: match err {
                    ResolveError::Builtin { resolved, is_runtime_module } => {
                        Some(Builtin { resolved, is_runtime_module })
                    }
                    _ => None,
                },
                module_type: None,
                error: Some(error),
                package_json_path: None,
            }
        }
    }
}

#[napi(string_enum = "lowercase")]
pub enum ModuleType {
    Module,
    CommonJs,
    Json,
    Wasm,
    Addon,
}

impl From<oxc_resolver::ModuleType> for ModuleType {
    fn from(value: oxc_resolver::ModuleType) -> Self {
        match value {
            oxc_resolver::ModuleType::Module => Self::Module,
            oxc_resolver::ModuleType::CommonJs => Self::CommonJs,
            oxc_resolver::ModuleType::Json => Self::Json,
            oxc_resolver::ModuleType::Wasm => Self::Wasm,
            oxc_resolver::ModuleType::Addon => Self::Addon,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn sync(path: String, request: String) -> ResolveResult {
    let path = PathBuf::from(path);
    let resolver = Resolver::new(ResolveOptions::default());
    resolve(&resolver, &path, &request)
}

pub struct ResolveTask {
    resolver: Arc<Resolver>,
    directory: PathBuf,
    request: String,
}

#[napi]
impl Task for ResolveTask {
    type JsValue = ResolveResult;
    type Output = ResolveResult;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        Ok(resolve(&self.resolver, &self.directory, &self.request))
    }

    fn resolve(&mut self, _: napi::Env, result: Self::Output) -> napi::Result<Self::JsValue> {
        Ok(result)
    }
}

#[napi]
pub struct ResolverFactory {
    resolver: Arc<Resolver>,
}

#[napi]
impl ResolverFactory {
    #[napi(constructor)]
    pub fn new(options: Option<NapiResolveOptions>) -> Self {
        #[cfg(feature = "tracing-subscriber")]
        {
            tracing::init_tracing();
        }
        let options = options.map_or_else(ResolveOptions::default, Self::normalize_options);
        Self { resolver: Arc::new(Resolver::new(options)) }
    }

    #[napi]
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self { resolver: Arc::new(Resolver::new(ResolveOptions::default())) }
    }

    /// Clone the resolver using the same underlying cache.
    #[napi]
    pub fn clone_with_options(&self, options: NapiResolveOptions) -> Self {
        Self {
            resolver: Arc::new(self.resolver.clone_with_options(Self::normalize_options(options))),
        }
    }

    /// Clear the underlying cache.
    #[napi]
    pub fn clear_cache(&self) {
        self.resolver.clear_cache();
    }

    /// Synchronously resolve `specifier` at an absolute path to a `directory`.
    #[allow(clippy::needless_pass_by_value)]
    #[napi]
    pub fn sync(&self, directory: String, request: String) -> ResolveResult {
        let path = PathBuf::from(directory);
        resolve(&self.resolver, &path, &request)
    }

    /// Asynchronously resolve `specifier` at an absolute path to a `directory`.
    #[allow(clippy::needless_pass_by_value)]
    #[napi(js_name = "async")]
    pub fn resolve_async(&self, directory: String, request: String) -> AsyncTask<ResolveTask> {
        let path = PathBuf::from(directory);
        let resolver = self.resolver.clone();
        AsyncTask::new(ResolveTask { resolver, directory: path, request })
    }

    fn normalize_options(op: NapiResolveOptions) -> ResolveOptions {
        let default = ResolveOptions::default();
        // merging options
        ResolveOptions {
            cwd: None,
            tsconfig: op.tsconfig.map(|tsconfig| tsconfig.into()),
            alias: op
                .alias
                .map(|alias| {
                    alias
                        .into_iter()
                        .map(|(k, v)| {
                            let v = v
                                .into_iter()
                                .map(|item| match item {
                                    Some(path) => oxc_resolver::AliasValue::from(path),
                                    None => oxc_resolver::AliasValue::Ignore,
                                })
                                .collect();
                            (k, v)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(default.alias),
            alias_fields: op
                .alias_fields
                .map(|o| o.into_iter().map(|x| StrOrStrList(x).into()).collect::<Vec<_>>())
                .unwrap_or(default.alias_fields),
            condition_names: op.condition_names.unwrap_or(default.condition_names),
            enforce_extension: op
                .enforce_extension
                .map(|enforce_extension| enforce_extension.into())
                .unwrap_or(default.enforce_extension),
            exports_fields: op
                .exports_fields
                .map(|o| o.into_iter().map(|x| StrOrStrList(x).into()).collect::<Vec<_>>())
                .unwrap_or(default.exports_fields),
            imports_fields: op
                .imports_fields
                .map(|o| o.into_iter().map(|x| StrOrStrList(x).into()).collect::<Vec<_>>())
                .unwrap_or(default.imports_fields),
            extension_alias: op
                .extension_alias
                .map(|extension_alias| extension_alias.into_iter().collect::<Vec<_>>())
                .unwrap_or(default.extension_alias),
            extensions: op.extensions.unwrap_or(default.extensions),
            fallback: op
                .fallback
                .map(|fallback| {
                    fallback
                        .into_iter()
                        .map(|(k, v)| {
                            let v = v
                                .into_iter()
                                .map(|item| match item {
                                    Some(path) => oxc_resolver::AliasValue::from(path),
                                    None => oxc_resolver::AliasValue::Ignore,
                                })
                                .collect();
                            (k, v)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or(default.fallback),
            fully_specified: op.fully_specified.unwrap_or(default.fully_specified),
            main_fields: op
                .main_fields
                .map(|o| StrOrStrList(o).into())
                .unwrap_or(default.main_fields),
            main_files: op.main_files.unwrap_or(default.main_files),
            modules: op.modules.map(|o| StrOrStrList(o).into()).unwrap_or(default.modules),
            resolve_to_context: op.resolve_to_context.unwrap_or(default.resolve_to_context),
            prefer_relative: op.prefer_relative.unwrap_or(default.prefer_relative),
            prefer_absolute: op.prefer_absolute.unwrap_or(default.prefer_absolute),
            restrictions: op
                .restrictions
                .map(|restrictions| {
                    restrictions
                        .into_iter()
                        .map(|restriction| restriction.into())
                        .collect::<Vec<_>>()
                })
                .unwrap_or(default.restrictions),
            roots: op
                .roots
                .map(|roots| roots.into_iter().map(PathBuf::from).collect::<Vec<_>>())
                .unwrap_or(default.roots),
            symlinks: op.symlinks.unwrap_or(default.symlinks),
            builtin_modules: op.builtin_modules.unwrap_or(default.builtin_modules),
            module_type: op.module_type.unwrap_or(default.module_type),
            allow_package_exports_in_directory_resolve: op
                .allow_package_exports_in_directory_resolve
                .unwrap_or(default.allow_package_exports_in_directory_resolve),
            #[cfg(feature = "yarn_pnp")]
            yarn_pnp: default.yarn_pnp,
        }
    }
}
