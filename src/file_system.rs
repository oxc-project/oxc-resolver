use std::{
    fs, io,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use pnp::fs::{LruZipCache, VPath, VPathInfo, ZipCache};

use crate::ResolveError;

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

    /// Reads a file while bypassing the system cache.
    ///
    /// This is useful in scenarios where the file content is already cached in memory
    /// and you want to avoid the overhead of using the system cache.
    ///
    /// # Errors
    ///
    /// * See [std::fs::read_to_string]
    fn read_to_string_bypass_system_cache(&self, path: &Path) -> io::Result<String> {
        self.read_to_string(path)
    }

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
    /// * Returns an error of [`ResolveError::IOError`] kind if there is an IO error invoking [`std::fs::read_link`].
    /// * Returns an error of [`ResolveError::PathNotSupported`] kind if the symlink target cannot be represented
    ///   as a path that can be consumed by the `import`/`require` syntax of Node.js.
    ///   Caller should not try to follow the symlink in this case.
    ///
    /// See [std::fs::read_link]
    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError>;
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

#[cfg(target_os = "windows")]
impl From<crate::windows::SymlinkMetadata> for FileMetadata {
    fn from(value: crate::windows::SymlinkMetadata) -> Self {
        Self::new(value.is_file, value.is_dir, value.is_symlink)
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
    /// See [std::io::ErrorKind::InvalidData]
    #[inline]
    pub fn validate_string(bytes: Vec<u8>) -> io::Result<String> {
        // `simdutf8` is faster than `std::str::from_utf8` which `fs::read_to_string` uses internally
        if simdutf8::basic::from_utf8(&bytes).is_err() {
            // Same error as `fs::read_to_string` produces (`io::Error::INVALID_UTF8`)
            #[cold]
            fn invalid_utf8_error() -> io::Error {
                io::Error::new(io::ErrorKind::InvalidData, "stream did not contain valid UTF-8")
            }
            return Err(invalid_utf8_error());
        }
        // SAFETY: `simdutf8` has ensured it's a valid UTF-8 string
        Ok(unsafe { String::from_utf8_unchecked(bytes) })
    }

    /// # Errors
    ///
    /// See [std::fs::read_to_string]
    pub fn read_to_string(path: &Path) -> io::Result<String> {
        let bytes = std::fs::read(path)?;
        Self::validate_string(bytes)
    }

    /// # Errors
    ///
    /// See [std::fs::metadata]
    #[inline]
    pub fn metadata(path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let result = crate::windows::symlink_metadata(path)?;
                if result.is_symlink {
                    return fs::metadata(path).map(FileMetadata::from);
                }
                Ok(result.into())
            } else if #[cfg(target_os = "linux")] {
                use rustix::fs::{AtFlags, CWD, FileType, StatxFlags};
                let statx = rustix::fs::statx(CWD, path, AtFlags::STATX_DONT_SYNC, StatxFlags::TYPE)?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
            } else {
                fs::metadata(path).map(FileMetadata::from)
            }
        }
    }

    /// # Errors
    ///
    /// See [std::fs::symlink_metadata]
    #[inline]
    pub fn symlink_metadata(path: &Path) -> io::Result<FileMetadata> {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                Ok(crate::windows::symlink_metadata(path)?.into())
            } else if #[cfg(target_os = "linux")] {
                use rustix::fs::{AtFlags, CWD, FileType, StatxFlags};
                let statx = rustix::fs::statx(
                    CWD,
                    path,
                    AtFlags::STATX_DONT_SYNC | AtFlags::SYMLINK_NOFOLLOW,
                    StatxFlags::TYPE,
                )?;
                let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
            } else {
                fs::symlink_metadata(path).map(FileMetadata::from)
            }
        }
    }

    /// # Errors
    ///
    /// See [std::fs::read_link]
    #[inline]
    pub fn read_link(path: &Path) -> Result<PathBuf, ResolveError> {
        let path = fs::read_link(path)?;
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                crate::windows::strip_windows_prefix(path)
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

    fn read_to_string_bypass_system_cache(&self, path: &Path) -> io::Result<String> {
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
        #[cfg(target_os = "macos")]
        {
            use libc::F_NOCACHE;
            use std::{io::Read, os::unix::fs::OpenOptionsExt};
            let mut fd = fs::OpenOptions::new().read(true).custom_flags(F_NOCACHE).open(path)?;
            let meta = fd.metadata()?;
            #[allow(clippy::cast_possible_truncation)]
            let mut buffer = Vec::with_capacity(meta.len() as usize);
            fd.read_to_end(&mut buffer)?;
            Self::validate_string(buffer)
        }
        #[cfg(target_os = "linux")]
        {
            use std::{io::Read, os::fd::AsRawFd};
            // Avoid `O_DIRECT` on Linux: it requires page-aligned buffers and aligned offsets,
            // which is incompatible with a regular Vec-based read and many CI filesystems.
            let mut fd = fs::OpenOptions::new().read(true).open(path)?;
            // Best-effort hint to avoid polluting the page cache.
            // SAFETY: `fd` is valid and `posix_fadvise` is safe.
            let _ = unsafe { libc::posix_fadvise(fd.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
            let meta = fd.metadata();
            let mut buffer = meta.ok().map_or_else(Vec::new, |meta| {
                #[allow(clippy::cast_possible_truncation)]
                Vec::with_capacity(meta.len() as usize)
            });
            fd.read_to_end(&mut buffer)?;
            Self::validate_string(buffer)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Self::read_to_string(path)
        }
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

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
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

#[test]
fn file_metadata_getters() {
    let file_meta = FileMetadata::new(true, false, false);
    assert!(file_meta.is_file());
    assert!(!file_meta.is_dir());
    assert!(!file_meta.is_symlink());

    let dir_meta = FileMetadata::new(false, true, false);
    assert!(!dir_meta.is_file());
    assert!(dir_meta.is_dir());
    assert!(!dir_meta.is_symlink());

    let symlink_meta = FileMetadata::new(false, false, true);
    assert!(!symlink_meta.is_file());
    assert!(!symlink_meta.is_dir());
    assert!(symlink_meta.is_symlink());
}
