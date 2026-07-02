use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use cfg_if::cfg_if;
#[cfg(feature = "yarn_pnp")]
use pnp::fs::{LruZipCache, VPath, VPathInfo, ZipCache};
#[cfg(feature = "yarn_pnp")]
use std::sync::Arc;

use crate::ResolveError;

/// File System abstraction used for `ResolverGeneric`
pub trait FileSystem: Send + Sync {
    #[cfg(feature = "yarn_pnp")]
    fn new(yarn_pnp: bool) -> Self
    where
        Self: Sized;

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self
    where
        Self: Sized;

    /// See [std::fs::read]
    ///
    /// # Errors
    ///
    /// * See [std::fs::read]
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// See [std::fs::read_to_string]
    ///
    /// # Errors
    ///
    /// * See [std::fs::read_to_string]
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
    /// * Returns an error of [`ResolveError::IOError`] kind if there is an IO error invoking [`std::fs::read_link`].
    /// * Returns an error of [`ResolveError::PathNotSupported`] kind if the symlink target cannot be represented
    ///   as a path that can be consumed by the `import`/`require` syntax of Node.js.
    ///   Caller should not try to follow the symlink in this case.
    ///
    /// See [std::fs::read_link]
    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError>;

    /// Returns the canonical, absolute form of a path with all intermediate components normalized.
    ///
    /// # Errors
    ///
    /// See [std::fs::canonicalize]
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// List a directory's entries with each entry's `lstat`-equivalent kind (`None` when the
    /// kind is not known from the directory stream and must be queried individually).
    ///
    /// This powers the resolver's per-directory listing cache: one `read_dir` answers
    /// existence and symlink-ness for every child, replacing per-candidate metadata calls.
    /// The default implementation returns [`io::ErrorKind::Unsupported`], which disables the
    /// listing cache and keeps per-path metadata behavior for custom file systems.
    ///
    /// # Errors
    ///
    /// * Any error from reading the directory. Callers treat an error as "listing
    ///   unavailable", never as "directory empty".
    fn read_dir(&self, path: &Path) -> io::Result<Vec<(OsString, Option<FileMetadata>)>> {
        let _ = path;
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
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

impl From<fs::FileType> for FileMetadata {
    // `FileType::is_file` is exactly `Metadata::is_file`, and this conversion must classify
    // paths identically to the `lstat`-based `From<fs::Metadata>` above (a socket/fifo is
    // neither file nor dir in both). `!is_dir()` would diverge for those.
    #[allow(clippy::filetype_is_file)]
    fn from(file_type: fs::FileType) -> Self {
        Self::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink())
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
#[derive(Clone)]
pub struct FileSystemOs;

#[cfg(feature = "yarn_pnp")]
#[derive(Clone)]
pub struct FileSystemOs {
    // `Arc` so `FileSystemOs` is cheaply `Clone`: clones share the same pnp zip cache.
    pnp_lru: Arc<LruZipCache<Vec<u8>>>,
    yarn_pnp: bool,
}

impl std::fmt::Debug for FileSystemOs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSystemOs").finish()
    }
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
                match rustix::fs::statx(CWD, path, AtFlags::STATX_DONT_SYNC, StatxFlags::TYPE) {
                    Ok(statx) => {
                        let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                        Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
                    }
                    Err(rustix::io::Errno::NOSYS) => {
                        // statx is not available (kernel < 4.11), fall back to fs::metadata
                        fs::metadata(path).map(FileMetadata::from)
                    }
                    Err(err) => Err(err.into()),
                }
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
                match rustix::fs::statx(CWD, path, AtFlags::SYMLINK_NOFOLLOW, StatxFlags::TYPE) {
                    Ok(statx) => {
                        let file_type = FileType::from_raw_mode(statx.stx_mode.into());
                        Ok(FileMetadata::new(file_type.is_file(), file_type.is_dir(), file_type.is_symlink()))
                    }
                    Err(rustix::io::Errno::NOSYS) => {
                        // statx is not available (kernel < 4.11), fall back to fs::symlink_metadata
                        fs::symlink_metadata(path).map(FileMetadata::from)
                    }
                    Err(err) => Err(err.into()),
                }
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

    /// # Errors
    ///
    /// See [std::fs::canonicalize]
    #[inline]
    pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
        fs::canonicalize(path)
    }
}

impl FileSystem for FileSystemOs {
    #[cfg(feature = "yarn_pnp")]
    fn new(yarn_pnp: bool) -> Self {
        Self { pnp_lru: Arc::new(LruZipCache::new(50, pnp::fs::open_zip_via_read_p)), yarn_pnp }
    }

    #[cfg(not(feature = "yarn_pnp"))]
    fn new() -> Self {
        Self
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => {
                            self.pnp_lru.read(info.physical_base_path(), info.zip_path)
                        }
                        VPath::Virtual(info) => fs::read(info.physical_base_path()),
                        VPath::Native(path) => fs::read(path),
                    }
                }
            }
        }
        fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        Self::validate_string(bytes)
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
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    // Mirror `metadata`'s virtual-path translation. Without it, `lstat`-ing a
                    // virtual or zip path directly stats a path that may not physically exist, so
                    // canonicalization cannot detect symlinks correctly under Yarn PnP. Zip entries
                    // are never symlinks, so reuse the same file-type lookup as `metadata`.
                    return match VPath::from(path)? {
                        VPath::Zip(info) => self
                            .pnp_lru
                            .file_type(info.physical_base_path(), info.zip_path)
                            .map(FileMetadata::from),
                        VPath::Virtual(info) => {
                            Self::symlink_metadata(&info.physical_base_path())
                        }
                        VPath::Native(path) => Self::symlink_metadata(&path),
                    }
                }
            }
        }
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

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => Self::canonicalize(&info.physical_base_path().join(info.zip_path)),
                        VPath::Virtual(info) => Self::canonicalize(&info.physical_base_path()),
                        VPath::Native(path) => Self::canonicalize(&path),
                    }
                }
            }
        }
        Self::canonicalize(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<(OsString, Option<FileMetadata>)>> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.yarn_pnp {
                    // Zip and virtual paths have no directory stream; keep per-path metadata
                    // behavior under Yarn PnP.
                    return Err(io::Error::from(io::ErrorKind::Unsupported));
                }
            }
        }
        // `DirEntry::file_type` comes from the directory stream (`d_type` on unix, find data
        // on windows); the standard library falls back to a metadata call only when the
        // filesystem reports an unknown type, matching what a per-entry probe would cost.
        fs::read_dir(path)?
            .map(|entry| {
                entry.map(|entry| (entry.file_name(), entry.file_type().ok().map(Into::into)))
            })
            .collect()
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
