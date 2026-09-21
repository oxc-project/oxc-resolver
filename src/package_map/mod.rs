//! Experimental [Node.js package map] resolution selected through `NODE_OPTIONS`.
//!
//! A package map contains a `packages` object whose opaque package IDs map to a required `url` and
//! an optional `dependencies` object. Entry URLs are resolved from the configured map path.
//! Multiple IDs resolving to the same path remain ambiguous, as required for [multiple packages
//! sharing one URL].
//!
//! The resolver API does not propagate a package ID between resolutions, so it uses the
//! specification's path-based fallback to identify the importer. Selecting a dependency follows
//! one map edge before regular resolution resumes; package-map dependency cycles are not detected,
//! matching the specification's limitation.
//!
//! Maps are loaded lazily because resolver construction cannot return errors, and are reloaded by
//! [`Resolver::clear_cache`](crate::Resolver::clear_cache).
//!
//! [Node.js package map]: https://nodejs.org/api/packages.html#package-maps
//! [multiple packages sharing one URL]: https://nodejs.org/api/packages.html#multiple-packages-for-the-same-url

mod cache;
mod map;
mod node_options;
mod resolver;
#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

pub use cache::PackageMapCache;
