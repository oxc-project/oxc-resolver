extern crate napi;
extern crate napi_derive;
extern crate oxc_resolver;

use std::path::{Path, PathBuf};

use napi_derive::napi;
use oxc_resolver::{ResolveOptions, Resolver};

use self::options::NapiResolveOptions;

mod options;

#[napi(object)]
pub struct ResolveResult {
    pub path: Option<String>,
    pub error: Option<String>,
}

#[napi]
pub struct ResolverFactory {
    resolver: Resolver,
}

#[napi]
impl ResolverFactory {
    #[napi(constructor)]
    pub fn new(op: NapiResolveOptions) -> Self {
        let default_options = ResolveOptions::default();
        // merging options
        let finalize_options = ResolveOptions {
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
                .unwrap_or(default_options.alias),
            alias_fields: op.alias_fields.unwrap_or(default_options.alias_fields),
            condition_names: op.condition_names.unwrap_or(default_options.condition_names),
            description_files: op.description_files.unwrap_or(default_options.description_files),
            enforce_extension: op
                .enforce_extension
                .map(|enforce_extension| enforce_extension.into())
                .unwrap_or(default_options.enforce_extension),
            exports_fields: op.exports_fields.unwrap_or(default_options.exports_fields),
            extension_alias: op
                .extension_alias
                .map(|extension_alias| extension_alias.into_iter().collect::<Vec<_>>())
                .unwrap_or(default_options.extension_alias),
            extensions: op.extensions.unwrap_or(default_options.extensions),
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
                .unwrap_or(default_options.fallback),
            fully_specified: op.fully_specified.unwrap_or(default_options.fully_specified),
            main_fields: op.main_fields.unwrap_or(default_options.main_fields),
            main_files: op.main_files.unwrap_or(default_options.main_files),
            modules: op.modules.unwrap_or(default_options.modules),
            resolve_to_context: op.resolve_to_context.unwrap_or(default_options.resolve_to_context),
            prefer_relative: op.prefer_relative.unwrap_or(default_options.prefer_relative),
            prefer_absolute: op.prefer_absolute.unwrap_or(default_options.prefer_absolute),
            restrictions: op
                .restrictions
                .map(|restrictions| {
                    restrictions
                        .into_iter()
                        .map(|restriction| restriction.into())
                        .collect::<Vec<_>>()
                })
                .unwrap_or(default_options.restrictions),
            roots: op
                .roots
                .map(|roots| roots.into_iter().map(PathBuf::from).collect::<Vec<_>>())
                .unwrap_or(default_options.roots),
            symlinks: op.symlinks.unwrap_or(default_options.symlinks),
            builtin_modules: op.builtin_modules.unwrap_or(default_options.builtin_modules),
        };
        Self { resolver: Resolver::new(finalize_options) }
    }
    #[napi]
    pub fn default() -> Self {
        let default_options = ResolveOptions::default();
        Self { resolver: Resolver::new(default_options) }
    }

    #[allow(clippy::needless_pass_by_value)]
    #[napi]
    pub fn sync(&self, path: String, request: String) -> ResolveResult {
        let path = PathBuf::from(path);
        resolve(&self.resolver, &path, &request)
    }
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
