use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::ResolveError;

use super::{map::PackageMap, node_options::package_map_path_from_node_options};

type CachedPackageMap = Result<Option<Arc<PackageMap>>, (PathBuf, ResolveError)>;

#[derive(Default)]
pub struct PackageMapCache {
    value: RwLock<Option<CachedPackageMap>>,
}

impl PackageMapCache {
    pub(super) fn get_or_init(
        &self,
        init: impl FnOnce(&Path) -> Result<PackageMap, ResolveError>,
    ) -> CachedPackageMap {
        if let Some(value) = self.value.read().expect("package map cache was poisoned").as_ref() {
            return value.clone();
        }

        let mut value = self.value.write().expect("package map cache was poisoned");
        if let Some(value) = value.as_ref() {
            return value.clone();
        }

        let initialized = package_map_path().map_or(Ok(None), |path| {
            init(&path)
                .map(|package_map| Some(Arc::new(package_map)))
                .map_err(|error| (path, error))
        });
        *value = Some(initialized.clone());
        initialized
    }

    pub fn clear(&self) {
        *self.value.write().expect("package map cache was poisoned") = None;
    }
}

fn package_map_path() -> Option<PathBuf> {
    let node_options = std::env::var("NODE_OPTIONS").ok()?;
    let cwd = std::env::current_dir().ok()?;
    package_map_path_from_node_options(&node_options, &cwd)
}
