extern crate napi;
extern crate napi_derive;
extern crate oxc_resolver;
use std::path::{Path, PathBuf};

use napi_derive::napi;
mod options;
use oxc_resolver::{ResolveOptions, Resolver};

use self::options::NapiResolveOptions;

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
    pub fn new(#[napi(ts_arg_type = "ResolveOptions")] op: NapiResolveOptions) -> Self {
        let default_options = ResolveOptions::default();
        Self { resolver: Resolver::new(default_options) }
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
