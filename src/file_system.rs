use cfg_if::cfg_if;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
    pub fn new(is_file: bool, is_dir: bool, is_symlink: bool) -> Self {
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

impl FileSystem for FileSystemOs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        cfg_if! {
            if #[cfg(feature = "yarn_pnp")] {
                match VPath::from(path)? {
                    VPath::Zip(info) => {
                        self.pnp_lru.read_to_string(info.physical_base_path(), info.zip_path)
                    }
                    VPath::Virtual(info) => fs::read_to_string(info.physical_base_path()),
                    VPath::Native(path) => fs::read_to_string(path),
                }
            } else {
                fs::read_to_string(path)
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
                        dunce::canonicalize(info.physical_base_path().join(info.zip_path))
                    }
                    VPath::Virtual(info) => dunce::canonicalize(info.physical_base_path()),
                    VPath::Native(path) => dunce::canonicalize(path),
                }
            } else if #[cfg(windows)] {
                dunce::canonicalize(path)
            } else {
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
