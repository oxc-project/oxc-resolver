use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use pnp::fs::{LruZipCache, VPath, VPathInfo, ZipCache};

#[cfg(windows)]
const UNC_PATH_PREFIX: &str = "\\\\?\\UNC\\";
#[cfg(windows)]
const LONG_PATH_PREFIX: &str = "\\\\?\\";

/// File System abstraction used for `ResolverGeneric`
pub trait FileSystem: Send + Sync {
    /// See [std::fs::read_to_string]
    ///
    /// # Errors
    ///
    /// * See [std::fs::read_to_string]
    /// ## Warning
    /// Use `&Path` instead of a generic `P: AsRef<Path>` here,
    /// because object safety requirements, it is especially useful, when
    /// you want to store multiple `dyn FileSystem` in a `Vec` or use a `ResolverGeneric<Fs>` in
    /// napi env.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// See [std::fs::metadata]
    ///
    /// # Errors
    /// See [std::fs::metadata]
    /// ## Warning
    /// Use `&Path` instead of a generic `P: AsRef<Path>` here,
    /// because object safety requirements, it is especially useful, when
    /// you want to store multiple `dyn FileSystem` in a `Vec` or use a `ResolverGeneric<Fs>` in
    /// napi env.
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// See [std::fs::symlink_metadata]
    ///
    /// # Errors
    ///
    /// See [std::fs::symlink_metadata]
    /// ## Warning
    /// Use `&Path` instead of a generic `P: AsRef<Path>` here,
    /// because object safety requirements, it is especially useful, when
    /// you want to store multiple `dyn FileSystem` in a `Vec` or use a `ResolverGeneric<Fs>` in
    /// napi env.
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// See [std::fs::canonicalize]
    ///
    /// # Errors
    ///
    /// See [std::fs::read_link]
    /// ## Warning
    /// Use `&Path` instead of a generic `P: AsRef<Path>` here,
    /// because object safety requirements, it is especially useful, when
    /// you want to store multiple `dyn FileSystem` in a `Vec` or use a `ResolverGeneric<Fs>` in
    /// napi env.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
}

/// Metadata information about a file
#[derive(Debug, Clone, Copy)]
pub struct FileMetadata {
    pub(crate) is_file: bool,
    pub(crate) is_dir: bool,
    pub(crate) is_symlink: bool,
}

impl FileMetadata {
    #[must_use]
    pub const fn new(is_file: bool, is_dir: bool, is_symlink: bool) -> Self {
        Self { is_file, is_dir, is_symlink }
    }
}

#[cfg(feature = "yarn_pnp")]
impl From<pnp::fs::FileType> for FileMetadata {
    fn from(value: pnp::fs::FileType) -> Self {
        Self::new(value == pnp::fs::FileType::File, value == pnp::fs::FileType::Directory, false)
    }
}

impl From<fs::Metadata> for FileMetadata {
    fn from(metadata: fs::Metadata) -> Self {
        Self::new(metadata.is_file(), metadata.is_dir(), metadata.is_symlink())
    }
}

/// Operating System
#[cfg(feature = "yarn_pnp")]
pub struct FileSystemOs {
    pnp_lru: LruZipCache<Vec<u8>>,
}

#[cfg(not(feature = "yarn_pnp"))]
pub struct FileSystemOs;

impl Default for FileSystemOs {
    fn default() -> Self {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                Self { pnp_lru: LruZipCache::new(50, pnp::fs::open_zip_via_read_p) }
            } else {
                Self
            }
        }
    }
}

fn read_to_string(path: &Path) -> io::Result<String> {
    // `simdutf8` is faster than `std::str::from_utf8` which `fs::read_to_string` uses internally
    let bytes = std::fs::read(path)?;
    if simdutf8::basic::from_utf8(&bytes).is_err() {
        // Same error as `fs::read_to_string` produces (`io::Error::INVALID_UTF8`)
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        ));
    }
    // SAFETY: `simdutf8` has ensured it's a valid UTF-8 string
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

impl FileSystem for FileSystemOs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => {
                        self.pnp_lru.read_to_string(info.physical_base_path(), info.zip_path)
                    }
                    VPath::Virtual(info) => read_to_string(&info.physical_base_path()),
                    VPath::Native(path) => read_to_string(&path),
                }
            } else {
                read_to_string(path)
            }
        }
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => self
                        .pnp_lru
                        .file_type(info.physical_base_path(), info.zip_path)
                        .map(FileMetadata::from),
                    VPath::Virtual(info) => {
                        fs::metadata(info.physical_base_path()).map(FileMetadata::from)
                    }
                    VPath::Native(path) => fs::metadata(path).map(FileMetadata::from),
                }
            } else {
                fs::metadata(path).map(FileMetadata::from)
            }
        }
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        fs::symlink_metadata(path).map(FileMetadata::from)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => {
                        node_compatible_raw_canonicalize(info.physical_base_path().join(info.zip_path))
                    }
                    VPath::Virtual(info) => node_compatible_raw_canonicalize(info.physical_base_path()),
                    VPath::Native(path) => node_compatible_raw_canonicalize(path),
                }
            } else if #[cfg(windows)] {
                node_compatible_raw_canonicalize(path)
            } else {
                use std::path::Component;
                let mut path_buf = path.to_path_buf();
                loop {
                    let link = fs::read_link(&path_buf)?;
                    path_buf.pop();
                    if fs::symlink_metadata(&path_buf)?.is_symlink()
                    {
                      path_buf = self.canonicalize(path_buf.as_path())?;
                    }
                    for component in link.components() {
                        match component {
                            Component::ParentDir => {
                                path_buf.pop();
                            }
                            Component::Normal(seg) => {
                                #[cfg(target_family = "wasm")]
                                {
                                  // Need to trim the extra \0 introduces by https://github.com/nodejs/uvwasi/issues/262
                                  path_buf.push(seg.to_string_lossy().trim_end_matches('\0'));
                                }
                                #[cfg(not(target_family = "wasm"))]
                                {
                                  path_buf.push(seg);
                                }
                            }
                            Component::RootDir => {
                                path_buf = PathBuf::from("/");
                            }
                            Component::CurDir | Component::Prefix(_) => {}
                        }
                        if fs::symlink_metadata(&path_buf)?.is_symlink()
                        {
                          path_buf = self.canonicalize(path_buf.as_path())?;
                        }
                    }
                    if !fs::symlink_metadata(&path_buf)?.is_symlink() {
                        break;
                    }
                }
                Ok(path_buf)
            }
        }
    }
}

#[test]
fn metadata() {
    let meta = FileMetadata { is_file: true, is_dir: true, is_symlink: true };
    assert_eq!(
        format!("{meta:?}"),
        "FileMetadata { is_file: true, is_dir: true, is_symlink: true }"
    );
    let _ = meta;
}

fn node_compatible_raw_canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    cfg_if! {
        if #[cfg(windows)] {
            use std::path::{Component, Prefix};
            // same logic with https://github.com/libuv/libuv/blob/d4ab6fbba4669935a6bc23645372dfe4ac29ab39/src/win/fs.c#L2774-L2784
            let canonicalized = fs::canonicalize(path)?;
            let first_component = canonicalized.components().next();
            match first_component {
                Some(Component::Prefix(prefix)) => {
                    match prefix.kind() {
                        Prefix::VerbatimUNC(_, _) => {
                            Ok(canonicalized.to_str().and_then(|s| s.get(UNC_PATH_PREFIX.len()..)).map(PathBuf::from).unwrap_or(canonicalized))
                        }
                        Prefix::VerbatimDisk(_) => {
                            Ok(canonicalized.to_str().and_then(|s| s.get(LONG_PATH_PREFIX.len()..)).map(PathBuf::from).unwrap_or(canonicalized))
                        }
                        _ => {
                            Ok(canonicalized)
                        }
                    }
                }
                _ => Ok(canonicalized),
            }
        } else {
            fs::canonicalize(path)
        }
    }
}
