use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use pnp::fs::{LruZipCache, VPath, VPathInfo, ZipCache};

/// File System abstraction used for `ResolverGeneric`
pub trait FileSystem: Send + Sync {
    #[cfg(feature = "yarn_pnp")]
    fn new(yarn_pnp: bool) -> Self;

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self;

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

#[cfg(not(feature = "yarn_pnp"))]
pub struct FileSystemOs;

#[cfg(feature = "yarn_pnp")]
pub struct FileSystemOs {
    pnp_lru: LruZipCache<Vec<u8>>,
    yarn_pnp: bool,
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
            if #[cfg(target_os = "windows")] {
                match crate::windows::try_strip_windows_prefix(path) {
                    // We won't follow the link if we cannot represent its target properly.
                    Ok(p) | Err(crate::ResolveError::PathNotSupported(p)) => Ok(p),
                    _ => unreachable!(),
                }
            } else {
                Ok(path)
            }
        }
    }
}

impl FileSystem for FileSystemOs {
    #[cfg(feature = "yarn_pnp")]
    fn new(yarn_pnp: bool) -> Self {
        Self { pnp_lru: LruZipCache::new(50, pnp::fs::open_zip_via_read_p), yarn_pnp }
    }

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        Self
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => {
                            self.pnp_lru.read_to_string(info.physical_base_path(), info.zip_path)
                        }
                        VPath::Virtual(info) => Self::read_to_string(&info.physical_base_path()),
                        VPath::Native(path) => Self::read_to_string(&path),
                    }
                }
            }
        }
        Self::read_to_string(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => self
                            .pnp_lru
                            .file_type(info.physical_base_path(), info.zip_path)
                            .map(FileMetadata::from),
                        VPath::Virtual(info) => {
                            Self::metadata(&info.physical_base_path())
                        }
                        VPath::Native(path) => Self::metadata(&path),
                    }
                }
            }
        }
        Self::metadata(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        Self::symlink_metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => Self::read_link(&info.physical_base_path().join(info.zip_path)),
                        VPath::Virtual(info) => Self::read_link(&info.physical_base_path()),
                        VPath::Native(path) => Self::read_link(&path),
                    }
                }
            }
        }
        Self::read_link(path)
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
