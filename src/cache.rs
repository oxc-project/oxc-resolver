use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{Ctx, PackageJson, ResolveError, ResolveOptions, TsConfig};

#[allow(clippy::missing_errors_doc)] // trait impls should be free to return any typesafe error
pub trait Cache: Sized {
    type Cp: CachedPath + Debug + Clone + Eq + std::hash::Hash;
    type Pj: PackageJson + Debug;
    type Tc: TsConfig + Debug;

    /// Clears the cache.
    fn clear(&self);

    /// Returns the cached value for a given path.
    fn value(&self, path: &Path) -> Self::Cp;

    /// Returns the canonical version of a `path`, resolving all symbolic links.
    fn canonicalize(&self, path: &Self::Cp) -> Result<PathBuf, ResolveError>;

    /// Returns whether the given `path` points to a file.
    fn is_file(&self, path: &Self::Cp, ctx: &mut Ctx) -> bool;

    /// Returns whether the given `path` points to a file.
    fn is_dir(&self, path: &Self::Cp, ctx: &mut Ctx) -> bool;

    /// Returns the package.json stored in the given directory, if one exists.
    ///
    /// `path` is the path to a directory from which the `package.json` will be
    /// read.
    #[allow(clippy::type_complexity)]
    fn get_package_json(
        &self,
        path: &Self::Cp,
        options: &ResolveOptions,
        ctx: &mut Ctx,
    ) -> Result<Option<(Self::Cp, Arc<Self::Pj>)>, ResolveError>;

    /// Returns the tsconfig stored in the given path.
    ///
    /// `path` is either the path to a tsconfig (with or without `.json`
    /// extension) or a directory from which the `tsconfig.json` will be read.
    ///
    /// `callback` can be used for modifying the returned tsconfig with
    /// `extends`.
    fn get_tsconfig<F: FnOnce(&mut Self::Tc) -> Result<(), ResolveError>>(
        &self,
        root: bool,
        path: &Path,
        callback: F,
    ) -> Result<Arc<Self::Tc>, ResolveError>;
}

#[allow(clippy::missing_errors_doc)] // trait impls should be free to return any typesafe error
pub trait CachedPath: Sized {
    fn path(&self) -> &Path;

    fn to_path_buf(&self) -> PathBuf;

    fn parent(&self) -> Option<&Self>;

    fn is_node_modules(&self) -> bool;

    fn inside_node_modules(&self) -> bool;

    fn module_directory<C: Cache<Cp = Self>>(
        &self,
        module_name: &str,
        cache: &C,
        ctx: &mut Ctx,
    ) -> Option<Self>;

    fn cached_node_modules<C: Cache<Cp = Self>>(&self, cache: &C, ctx: &mut Ctx) -> Option<Self>;

    /// Find package.json of a path by traversing parent directories.
    #[allow(clippy::type_complexity)]
    fn find_package_json<C: Cache<Cp = Self>>(
        &self,
        options: &ResolveOptions,
        cache: &C,
        ctx: &mut Ctx,
    ) -> Result<Option<(Self, Arc<C::Pj>)>, ResolveError>;

    #[must_use]
    fn add_extension<C: Cache<Cp = Self>>(&self, ext: &str, cache: &C) -> Self;

    #[must_use]
    fn replace_extension<C: Cache<Cp = Self>>(&self, ext: &str, cache: &C) -> Self;

    /// Returns a new path by resolving the given subpath (including "." and
    /// ".." components) with this path.
    #[must_use]
    fn normalize_with<C: Cache<Cp = Self>, P: AsRef<Path>>(&self, subpath: P, cache: &C) -> Self;

    #[must_use]
    fn normalize_root<C: Cache<Cp = Self>>(&self, _cache: &C) -> Self;
}
