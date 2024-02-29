extern crate napi;
extern crate napi_derive;
extern crate oxc_resolver;

use std::path::{Path, PathBuf};

use napi_derive::napi;
use oxc_resolver::{ResolveOptions, Resolver};

use self::options::{NapiResolveOptions, StrOrStrList};

mod options;

#[napi(object)]
pub struct ResolveResult {
    pub path: Option<String>,
    pub error: Option<String>,
}

fn resolve(resolver: &Resolver, path: &Path, request: &str) -> ResolveResult {
    match resolver.resolve(path, request) {
        Ok(resolution) => ResolveResult {
            path: Some(resolution.full_path().to_string_lossy().to_string()),
            error: None,
        },
        Err(err) => ResolveResult { path: None, error: Some(err.to_string()) },
    }
}

#[allow(clippy::needless_pass_by_value)]
#[napi]
pub fn sync(path: String, request: String) -> ResolveResult {
    let path = PathBuf::from(path);
    let resolver = Resolver::new(ResolveOptions::default());
    resolve(&resolver, &path, &request)
}

#[napi]
pub struct ResolverFactory {
    resolver: Resolver,
}

#[napi]
impl ResolverFactory {
    #[napi(constructor)]
    pub fn new(options: NapiResolveOptions) -> Self {
        Self { resolver: Resolver::new(Self::normalize_options(options)) }
    }

    #[napi]
    pub fn default() -> Self {
        let default_options = ResolveOptions::default();
        Self { resolver: Resolver::new(default_options) }
    }

    /// Clone the resolver using the same underlying cache.
    #[napi]
    pub fn clone_with_options(&self, options: NapiResolveOptions) -> Self {
        Self { resolver: self.resolver.clone_with_options(Self::normalize_options(options)) }
    }

    /// Clear the underlying cache.
    #[napi]
    pub fn clear_cache(&self) {
        self.resolver.clear_cache();
    }

    #[allow(clippy::needless_pass_by_value)]
    #[napi]
    pub fn sync(&self, path: String, request: String) -> ResolveResult {
        let path = PathBuf::from(path);
        resolve(&self.resolver, &path, &request)
    }

    fn normalize_options(op: NapiResolveOptions) -> ResolveOptions {
        let default = ResolveOptions::default();
        // merging options
        ResolveOptions {
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
                                    Some(path) => oxc_resolver::AliasValue::Path(path),
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
            description_files: op.description_files.unwrap_or(default.description_files),
            enforce_extension: op
                .enforce_extension
                .map(|enforce_extension| enforce_extension.into())
                .unwrap_or(default.enforce_extension),
            exports_fields: op
                .exports_fields
                .map(|o| o.into_iter().map(|x| StrOrStrList(x).into()).collect::<Vec<_>>())
                .unwrap_or(default.exports_fields),
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
                                    Some(path) => oxc_resolver::AliasValue::Path(path),
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
            reduce_memory_usage: op.reduce_memory_usage.unwrap_or(default.reduce_memory_usage),
        }
    }
}
