# Implementation Plan: Index-Based Parent Tree with Papaya + Generation Swapping

## Status: Phase 1 Complete ✅

The core new implementation is complete, tested, and all 158 tests pass. Both implementations (old Arc-based and new generation-based) coexist in the codebase.

### Completed:

- ✅ Added `arc-swap = "1.7"` dependency
- ✅ Created `PathNode` struct with u32 indices instead of Arc/Weak
- ✅ Created `CacheGeneration` struct with RwLock<Vec> + papaya HashMap
- ✅ Created `PathHandle` with index + Arc to generation
- ✅ Updated `Cache` struct with both old `paths` field and new `generation` field
- ✅ Implemented `cache.value_v2()` - generation-based path lookup with lock-free papaya
- ✅ Implemented `cache.clear_v2()` - atomic generation swapping via ArcSwap
- ✅ All code compiles successfully
- ✅ Added 7 comprehensive tests verifying the new implementation
- ✅ All 158 tests pass (151 existing + 7 new)

### Key Implementation Details:

- PathHandle is 12 bytes (u32 index + Arc pointer)
- Parent pointers are u32 indices (not Weak<Arc<...>>)
- Papaya HashMap provides lock-free path lookups
- RwLock<Vec> for node storage (concurrent reads, exclusive writes)
- Generation swapping ensures ongoing resolutions continue to work during clear_cache

### Test Coverage:

1. `test_value_v2_creates_handle` - Basic handle creation
2. `test_value_v2_parent_traversal` - Parent chain traversal via indices
3. `test_value_v2_deduplication` - Same path returns same index
4. `test_clear_v2_creates_new_generation` - Generation swapping works
5. `test_clear_v2_ongoing_resolution_safety` - **Critical**: Old handles continue working after clear_cache
6. `test_path_handle_equality` - Equality semantics
7. `test_node_modules_detection` - node_modules flag propagation

### Next Steps (Phase 2):

To complete the migration, you would:

1. Add conversion helpers between CachedPath and PathHandle
2. Migrate one high-level function to use PathHandle (e.g., a simple resolver method)
3. Gradually migrate more code
4. Eventually remove old Arc-based implementation
5. Rename `value_v2()` to `value()` and `clear_v2()` to `clear()`

---

# Implementation Plan: Index-Based Parent Tree with Papaya + Generation Swapping

**Goal**: Remove Arc from parent-pointing tree, keep papaya's lock-free benefits, ensure clear_cache safety

**Core Architecture**:

- PathHandle: u32 index + Arc to generation
- PathNode: parent as u32 index (not Weak)
- Papaya: lock-free path lookups
- ArcSwap: atomic generation swapping for clear_cache

## Core Design

```rust
use arc_swap::ArcSwap;

// PathHandle: cheap to clone (12 bytes)
#[derive(Clone)]
pub struct PathHandle {
    index: u32,
    generation: Arc<CacheGeneration>,
}

// PathNode: parent is just u32 index
struct PathNode {
    hash: u64,
    path: Box<Path>,
    parent_idx: Option<u32>,  // ← u32, not Weak<Arc<...>>!
    is_node_modules: bool,
    inside_node_modules: bool,
    meta: OnceLock<Option<FileMetadata>>,
    canonicalized_idx: Mutex<Result<Option<u32>, ResolveError>>,
    canonicalizing: AtomicU64,
    node_modules_idx: OnceLock<Option<u32>>,
    package_json: OnceLock<Option<Arc<PackageJson>>>,
    tsconfig: OnceLock<Option<Arc<TsConfig>>>,
}

// CacheGeneration: one snapshot of cache state
struct CacheGeneration {
    nodes: RwLock<Vec<PathNode>>,
    path_to_idx: papaya::HashMap<u64, u32, BuildHasherDefault<IdentityHasher>>,
}

// Cache: atomically swappable generation
pub struct Cache<Fs> {
    fs: Fs,
    generation: ArcSwap<CacheGeneration>,
    tsconfigs: papaya::HashMap<PathBuf, Arc<TsConfig>>,
}
```

## Phase 1: Add Dependencies

1. Add `arc-swap = "1.7"` to Cargo.toml
2. Keep existing `papaya` dependency
3. No other new dependencies needed

## Phase 2: Create New Types

1. Create `PathNode` struct (rename from CachedPathImpl)
   - Change `parent: Option<Weak<CachedPathImpl>>` to `parent_idx: Option<u32>`
   - Change `canonicalized: Mutex<Result<Weak<...>>>` to `canonicalized_idx: Mutex<Result<Option<u32>, ...>>`
   - Change `node_modules: OnceLock<Option<Weak<...>>>` to `node_modules_idx: OnceLock<Option<u32>>`

2. Create `CacheGeneration` struct
   - Move nodes storage: `RwLock<Vec<PathNode>>`
   - Move path lookup: `papaya::HashMap<u64, u32>` (hash to index)

3. Create new `PathHandle` struct
   - `index: u32`
   - `generation: Arc<CacheGeneration>`
   - Implement Clone (cheap Arc clone)

## Phase 3: Restructure Cache

1. Replace `paths: HashSet<CachedPath>` with `generation: ArcSwap<CacheGeneration>`
2. Keep `tsconfigs` at Cache level (or move into generation if needed)
3. Keep `fs: Fs` at Cache level
4. Initialize with empty generation in Cache::default()

## Phase 4: Implement Core Cache Methods

**cache.value(path) → PathHandle**:

```rust
pub fn value(&self, path: &Path) -> PathHandle {
    let hash = compute_hash(path);
    let gen = self.generation.load_full();  // Arc clone of generation

    // Fast path: lock-free lookup via papaya
    if let Some(&idx) = gen.path_to_idx.pin().get(&hash) {
        return PathHandle { index: idx, generation: gen };
    }

    // Slow path: need to insert
    let parent_idx = path.parent().map(|p| self.value(p).index);
    let node = PathNode::new(hash, path, parent_idx, ...);

    // Lock Vec for append
    let mut nodes = gen.nodes.write().unwrap();
    let idx = nodes.len() as u32;
    nodes.push(node);
    drop(nodes);

    // Lock-free insert into papaya
    gen.path_to_idx.pin().insert(hash, idx);

    PathHandle { index: idx, generation: gen }
}
```

**cache.get_node(handle) → access to PathNode**:

```rust
pub fn get_node(&self, handle: &PathHandle) -> impl Deref<Target = PathNode> {
    RwLockReadGuard::map(
        handle.generation.nodes.read().unwrap(),
        |vec| &vec[handle.index as usize]
    )
}
```

**cache.parent(handle) → Option<PathHandle>**:

```rust
pub fn parent(&self, handle: &PathHandle) -> Option<PathHandle> {
    let node = self.get_node(handle);
    node.parent_idx.map(|idx| PathHandle {
        index: idx,
        generation: handle.generation.clone(),
    })
}
```

**cache.clear() → atomic swap**:

```rust
pub fn clear(&self) {
    let new_gen = Arc::new(CacheGeneration {
        nodes: RwLock::new(Vec::new()),
        path_to_idx: papaya::HashMap::new(),
    });
    self.generation.store(new_gen);
    // Old generation stays alive via existing PathHandles

    self.tsconfigs.pin().clear();
}
```

## Phase 5: Update cached_path.rs

1. Remove `CachedPath(Arc<CachedPathImpl>)` wrapper
2. Update all methods to work with PathHandle + Cache reference
3. Update `find_package_json` to traverse via indices
4. Update `canonicalize_impl` to use indices

## Phase 6: Update cache_impl.rs

1. Replace HashSet operations with generation-based operations
2. Update all helper methods to use PathHandle
3. Ensure papaya HashMap is used for lookups
4. Ensure RwLock is used for Vec mutations

## Phase 7: Update lib.rs (~100+ locations)

1. Replace `CachedPath` with `PathHandle` in all signatures
2. Update all `.clone()` calls (still works, just clones Arc)
3. Update parent traversal: `iter::successors(Some(handle.clone()), |h| cache.parent(h))`
4. Pass cache reference where needed for node access
5. Update all path comparisons and operations

## Phase 8: Testing

1. Run existing test suite
2. Add test: concurrent resolution during clear_cache
3. Add test: old handles still valid after clear
4. Add test: parent traversal with indices
5. Add test: verify generation is freed when handles dropped
6. Run `test_memory_leak_arc_cycles` (should still pass)

## Phase 9: Benchmarking

1. Memory usage before/after (expect ~50% reduction)
2. Parent traversal speed (expect 2-3x improvement)
3. Overall resolution throughput
4. Ensure papaya lookups remain lock-free and fast

## Key Benefits

- ✅ Arc per generation (not per path) - 50% memory savings
- ✅ Parent pointers are u32 (not Weak) - faster traversal, no upgrade failures
- ✅ Papaya for lock-free path lookups - fast path stays fast
- ✅ RwLock only for Vec append (rare) - minimal contention
- ✅ clear_cache is atomic and safe - ongoing resolutions unaffected
- ✅ Automatic memory reclamation - generations freed when unused

## Trade-offs

- PathHandle is Clone (not Copy) - acceptable, 12-byte struct with Arc
- Need cache reference for node access - acceptable for internal API
- RwLock for Vec - acceptable, concurrent reads work fine
