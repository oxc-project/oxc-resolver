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
    /// See [std::fs::read]
    ///
    /// # Errors
    ///
    /// See [std::fs::read]
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

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
    /// * Returns an error of [`ResolveError::IOError`] kind if there is an IO error invoking [`std::fs::read_link`].
    /// * Returns an error of [`ResolveError::PathNotSupported`] kind if the symlink target cannot be represented
    ///   as a path that can be consumed by the `import`/`require` syntax of Node.js.
    ///   Caller should not try to follow the symlink in this case.
    ///
    /// See [std::fs::read_link]
    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError>;

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

pub struct FileSystemOptions {
    #[cfg(feature = "yarn_pnp")]
    pub enable_pnp: bool,
}

impl Default for FileSystemOptions {
    fn default() -> Self {
        Self {
            #[cfg(feature = "yarn_pnp")]
            enable_pnp: true,
        }
    }
}

/// Operating System
pub struct FileSystemOs {
    options: FileSystemOptions,
    #[cfg(feature = "yarn_pnp")]
    pnp_lru: LruZipCache<Vec<u8>>,
}

#[cfg(not(feature = "yarn_pnp"))]
pub struct FileSystemOs;

impl Default for FileSystemOs {
    fn default() -> Self {
        Self {
            options: FileSystemOptions::default(),
            #[cfg(feature = "yarn_pnp")]
            pnp_lru: LruZipCache::new(50, pnp::fs::open_zip_via_read_p),
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

fn buffer_to_string(bytes: Vec<u8>) -> io::Result<String> {
    // `simdutf8` is faster than `std::str::from_utf8` which `fs::read_to_string` uses internally
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
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        cfg_if! {
          if #[cfg(feature = "yarn_pnp")] {
            if self.options.enable_pnp {
                return match VPath::from(path)? {
                    VPath::Zip(info) => self.pnp_lru.read(info.physical_base_path(), info.zip_path),
                    VPath::Virtual(info) => std::fs::read(info.physical_base_path()),
                    VPath::Native(path) => std::fs::read(&path),
                }
            }
        }}

        std::fs::read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let buffer = self.read(path)?;
        buffer_to_string(buffer)
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

    fn read_link(&self, path: &Path) -> Result<PathBuf, ResolveError> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => Self::read_link(&info.physical_base_path().join(info.zip_path)),
                    VPath::Virtual(info) => Self::read_link(&info.physical_base_path()),
                    VPath::Native(path) => Self::read_link(&path),
                }
            } else {
                Self::try_read_link(path)
            }
        }
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                if self.options.enable_pnp {
                    return match VPath::from(path)? {
                        VPath::Zip(info) => {
                            dunce::canonicalize(info.physical_base_path().join(info.zip_path))
                        }
                        VPath::Virtual(info) => dunce::canonicalize(info.physical_base_path()),
                        VPath::Native(path) => dunce::canonicalize(path),
                    }
                }
            }
        }

        cfg_if! {
            if #[cfg(not(target_os = "wasi"))]{
                dunce::canonicalize(path)
            } else {
                use std::path::Component;
                let mut path_buf = path.to_path_buf();
                loop {
                    let link = fs::read_link(&path_buf)?;
                    path_buf.pop();
                    for component in link.components() {
                        match component {
                            Component::ParentDir => {
                                path_buf.pop();
                            }
                            Component::Normal(seg) => {
                                #[cfg(target_family = "wasm")]
                                // Need to trim the extra \0 introduces by https://github.com/nodejs/uvwasi/issues/262
                                {
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
