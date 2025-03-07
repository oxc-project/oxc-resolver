use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use pnp::fs::{LruZipCache, VPath, VPathInfo, ZipCache};

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

    /// Returns the resolution of a symbolic link.
    ///
    /// # Errors
    ///
    /// See [std::fs::read_link]
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;
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

    #[must_use]
    pub const fn is_file(self) -> bool {
        self.is_file
    }

    #[must_use]
    pub const fn is_dir(self) -> bool {
        self.is_dir
    }

    #[must_use]
    pub const fn is_symlink(self) -> bool {
        self.is_symlink
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

impl FileSystemOs {
    /// # Errors
    ///
    /// See [std::fs::read_to_string]
    pub fn read_to_string(path: &Path) -> io::Result<String> {
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

    /// # Errors
    ///
    /// See [std::fs::metadata]
    #[inline]
    pub fn metadata(path: &Path) -> io::Result<FileMetadata> {
        fs::metadata(path).map(FileMetadata::from)
    }

    /// # Errors
    ///
    /// See [std::fs::symlink_metadata]
    #[inline]
    pub fn symlink_metadata(path: &Path) -> io::Result<FileMetadata> {
        fs::symlink_metadata(path).map(FileMetadata::from)
    }

    /// # Errors
    ///
    /// See [std::fs::read_link]
    #[inline]
    pub fn read_link(path: &Path) -> io::Result<PathBuf> {
        let path = fs::read_link(path)?;
        cfg_if! {
            if #[cfg(windows)] {
                Ok(Self::strip_windows_prefix(path))
            } else {
                Ok(path)
            }
        }
    }

    pub fn strip_windows_prefix<P: AsRef<Path>>(path: P) -> PathBuf {
        const UNC_PATH_PREFIX: &[u8] = b"\\\\?\\UNC\\";
        const LONG_PATH_PREFIX: &[u8] = b"\\\\?\\";
        let path_bytes = path.as_ref().as_os_str().as_encoded_bytes();
        path_bytes
            .strip_prefix(UNC_PATH_PREFIX)
            .or_else(|| path_bytes.strip_prefix(LONG_PATH_PREFIX))
            .map_or_else(
                || path.as_ref().to_path_buf(),
                |p| {
                    // SAFETY: `as_encoded_bytes` ensures `p` is valid path bytes
                    unsafe { PathBuf::from(std::ffi::OsStr::from_encoded_bytes_unchecked(p)) }
                },
            )
    }
}

impl FileSystem for FileSystemOs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => {
                        self.pnp_lru.read_to_string(info.physical_base_path(), info.zip_path)
                    }
                    VPath::Virtual(info) => Self::read_to_string(&info.physical_base_path()),
                    VPath::Native(path) => Self::read_to_string(&path),
                }
            } else {
                Self::read_to_string(path)
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
                        Self::metadata(&info.physical_base_path())
                    }
                    VPath::Native(path) => Self::metadata(&path),
                }
            } else {
                Self::metadata(path)}
        }
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => Self::read_link(&info.physical_base_path().join(info.zip_path)),
                    VPath::Virtual(info) => Self::read_link(&info.physical_base_path()),
                    VPath::Native(path) => Self::read_link(&path),
                }
            } else {
                Self::read_link(path)
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
