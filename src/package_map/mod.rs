//! Implementation of experimental [Node.js package maps][node-package-maps].
//!
//! A package map is one static JSON file containing a `packages` object. Each key is an opaque,
//! unique package ID. Its value has a required `url` and an optional `dependencies` object that
//! maps the bare package name used by source code to another package ID. Package IDs therefore
//! identify dependency graph nodes independently of their locations and allow different importers
//! to resolve the same package name to different versions.
//!
//! Entry URLs are resolved from the package map's configured `NODE_OPTIONS` location into
//! filesystem paths. Explicit URLs must use the `file:` protocol. The resulting paths form the
//! index used to identify the package that owns an importer. Multiple IDs resolving to the same
//! owning path are retained as ambiguous, as required for [multiple packages sharing one
//! URL][shared-url].
//!
//! Package-map resolution is enabled automatically when `NODE_OPTIONS` contains Node's
//! `--experimental-package-map` option. The resolver API does not propagate a package ID between
//! resolutions, so it always uses the specification's path-based fallback to identify the
//! importer. The `NODE_OPTIONS` lookup, parsed map, and both successful and failed ownership
//! lookups are cached until [`Resolver::clear_cache`](crate::Resolver::clear_cache) is called.
//! Parsing itself is synchronous but deferred until the first applicable resolution because
//! resolver construction cannot return an error. Selecting a dependency follows one map edge
//! before regular resolution resumes; package-map dependency cycles are not detected, matching
//! the specification's limitation.
//!
//! The accessor logic is shared between two storage backends: little-endian systems borrow
//! strings directly from simd-json's input buffer, while big-endian systems store owned
//! [`compact_str::CompactString`]s parsed by serde-json.
//!
//! Package-manager configuration:
//! [pnpm](https://pnpm.io/settings#nodeexperimentalpackagemap),
//! [Yarn](https://yarnpkg.com/configuration/yarnrc#nodeExperimentalPackageMap).
//!
//! [node-package-maps]: https://nodejs.org/api/packages.html#package-maps
//! [shared-url]: https://nodejs.org/api/packages.html#multiple-packages-for-the-same-url

mod cache;
mod map;
mod node_options;
mod resolver;
#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

pub use cache::PackageMapCache;
