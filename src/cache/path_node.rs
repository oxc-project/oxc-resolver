// New index-based implementation for cached paths
// This will eventually replace CachedPath/CachedPathImpl

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::AtomicU64},
};

use once_cell::sync::OnceCell as OnceLock;

use crate::{FileMetadata, PackageJson, ResolveError, TsConfig};

/// Handle to a cached path - contains index into generation's storage
#[derive(Clone)]
pub struct PathHandle {
    pub(crate) index: u32,
    pub(crate) generation: Arc<CacheGeneration>,
}

impl std::fmt::Debug for PathHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathHandle")
            .field("index", &self.index)
            .field("path", &self.path())
            .finish()
    }
}

/// Storage for one "generation" of cached paths
pub struct CacheGeneration {
    pub(crate) nodes: parking_lot::RwLock<Vec<PathNode>>,
    pub(crate) path_to_idx:
        papaya::HashMap<u64, u32, std::hash::BuildHasherDefault<super::hasher::IdentityHasher>>,
}

impl CacheGeneration {
    pub fn new() -> Self {
        Self {
            nodes: parking_lot::RwLock::new(Vec::new()),
            path_to_idx: papaya::HashMap::builder()
                .hasher(std::hash::BuildHasherDefault::default())
                .resize_mode(papaya::ResizeMode::Blocking)
                .build(),
        }
    }
}

impl Default for CacheGeneration {
    fn default() -> Self {
        Self::new()
    }
}

/// Data for a single cached path node
pub struct PathNode {
    pub(crate) hash: u64,
    pub(crate) path: Box<Path>,
    pub(crate) parent_idx: Option<u32>,
    pub(crate) is_node_modules: bool,
    pub(crate) inside_node_modules: bool,
    pub(crate) meta: OnceLock<Option<FileMetadata>>,
    pub(crate) canonicalized_idx: Mutex<Result<Option<u32>, ResolveError>>,
    pub(crate) canonicalizing: AtomicU64,
    pub(crate) node_modules_idx: OnceLock<Option<u32>>,
    pub(crate) package_json: OnceLock<Option<Arc<PackageJson>>>,
    pub(crate) tsconfig: OnceLock<Option<Arc<TsConfig>>>,
}

impl PathNode {
    pub fn new(
        hash: u64,
        path: Box<Path>,
        parent_idx: Option<u32>,
        is_node_modules: bool,
        inside_node_modules: bool,
    ) -> Self {
        Self {
            hash,
            path,
            parent_idx,
            is_node_modules,
            inside_node_modules,
            meta: OnceLock::new(),
            canonicalized_idx: Mutex::new(Ok(None)),
            canonicalizing: AtomicU64::new(0),
            node_modules_idx: OnceLock::new(),
            package_json: OnceLock::new(),
            tsconfig: OnceLock::new(),
        }
    }
}

impl PathHandle {
    /// Get the path (returns owned PathBuf for simplicity)
    pub(crate) fn path(&self) -> PathBuf {
        let nodes = self.generation.nodes.read();
        nodes[self.index as usize].path.to_path_buf()
    }

    /// Get path as PathBuf
    pub(crate) fn to_path_buf(&self) -> PathBuf {
        self.path()
    }

    /// Get hash
    pub(crate) fn hash(&self) -> u64 {
        let nodes = self.generation.nodes.read();
        nodes[self.index as usize].hash
    }

    /// Get parent handle
    pub(crate) fn parent(&self) -> Option<Self> {
        let nodes = self.generation.nodes.read();
        nodes[self.index as usize]
            .parent_idx
            .map(|idx| PathHandle { index: idx, generation: self.generation.clone() })
    }

    /// Check if this is a node_modules directory
    pub(crate) fn is_node_modules(&self) -> bool {
        let nodes = self.generation.nodes.read();
        nodes[self.index as usize].is_node_modules
    }

    /// Check if path is inside node_modules
    pub(crate) fn inside_node_modules(&self) -> bool {
        let nodes = self.generation.nodes.read();
        nodes[self.index as usize].inside_node_modules
    }
}

impl PartialEq for PathHandle {
    fn eq(&self, other: &Self) -> bool {
        // Fast path: same generation and index
        if Arc::ptr_eq(&self.generation, &other.generation) && self.index == other.index {
            return true;
        }
        // Slow path: compare actual paths
        let nodes1 = self.generation.nodes.read();
        let nodes2 = other.generation.nodes.read();
        nodes1[self.index as usize].path.as_os_str()
            == nodes2[other.index as usize].path.as_os_str()
    }
}

impl Eq for PathHandle {}

impl std::hash::Hash for PathHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash().hash(state);
    }
}

#[path = "path_node_test.rs"]
#[cfg(test)]
mod path_node_test;
