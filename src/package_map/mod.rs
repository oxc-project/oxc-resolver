//! Experimental [Node.js package map] resolution selected through `NODE_OPTIONS`.
//!
//! Maps are loaded lazily because resolver construction cannot return errors, and are reloaded by
//! [`Resolver::clear_cache`](crate::Resolver::clear_cache).
//!
//! [Node.js package map]: https://nodejs.org/api/packages.html#package-maps

mod cache;
mod map;
mod node_options;
mod resolver;
#[cfg(target_endian = "big")]
mod serde;
#[cfg(target_endian = "little")]
mod simd;

pub use cache::PackageMapCache;
