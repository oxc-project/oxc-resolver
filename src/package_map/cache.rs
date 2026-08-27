use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, OnceLock},
};

use crate::ResolveError;

use super::{map::PackageMap, node_options::package_map_path_from_node_options};

pub struct PackageMapCache {
    pub(super) path: PathBuf,
    logical_value: OnceLock<Result<Arc<PackageMap>, ResolveError>>,
    canonical_value: OnceLock<Result<Arc<PackageMap>, ResolveError>>,
}

impl PackageMapCache {
    fn new(path: PathBuf) -> Self {
        Self { path, logical_value: OnceLock::new(), canonical_value: OnceLock::new() }
    }

    pub(super) const fn value(
        &self,
        symlinks: bool,
    ) -> &OnceLock<Result<Arc<PackageMap>, ResolveError>> {
        if symlinks { &self.canonical_value } else { &self.logical_value }
    }
}

static NODE_OPTIONS_PACKAGE_MAP_PATH: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let node_options = std::env::var("NODE_OPTIONS").ok()?;
    let cwd = std::env::current_dir().ok()?;
    package_map_path_from_node_options(&node_options, &cwd)
});

pub fn configure() -> Option<Box<PackageMapCache>> {
    NODE_OPTIONS_PACKAGE_MAP_PATH.as_ref().map(|path| Box::new(PackageMapCache::new(path.clone())))
}
