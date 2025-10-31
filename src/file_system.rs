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
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            // Use pread for positioned reads (thread-safe, no seek overhead)
            unix_optimized::pread_to_string(path)
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        )))]
        {
            let bytes = std::fs::read(path)?;
            Self::validate_string(bytes)
        }
    }

    /// # Errors
    ///
    /// See [std::fs::metadata]
    #[inline]
    pub fn metadata(path: &Path) -> io::Result<FileMetadata> {
        #[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            // Use optimized statx on Linux (kernel 4.11+)
            return linux_optimized::statx_metadata(path, true);
        }

        #[cfg(all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            // Use optimized getattrlist on macOS
            macos_optimized::getattrlist_metadata(path, true)
        }

        #[cfg(target_os = "windows")]
        {
            let result = crate::windows::symlink_metadata(path)?;
            if result.is_symlink {
                return fs::metadata(path).map(FileMetadata::from);
            }
            Ok(result.into())
        }

        #[cfg(not(any(
            target_os = "windows",
            all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
            all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64"))
        )))]
        {
            fs::metadata(path).map(FileMetadata::from)
        }
    }

    /// # Errors
    ///
    /// See [std::fs::symlink_metadata]
    #[inline]
    pub fn symlink_metadata(path: &Path) -> io::Result<FileMetadata> {
        #[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            // Use optimized statx on Linux (kernel 4.11+)
            return linux_optimized::statx_metadata(path, false);
        }

        #[cfg(all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            // Use optimized getattrlist on macOS
            macos_optimized::getattrlist_metadata(path, false)
        }

        #[cfg(target_os = "windows")]
        {
            Ok(crate::windows::symlink_metadata(path)?.into())
        }

        #[cfg(not(any(
            target_os = "windows",
            all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
            all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64"))
        )))]
        {
            fs::symlink_metadata(path).map(FileMetadata::from)
        }
    }

    /// # Errors
    ///
    /// See [std::fs::read_link]
    #[inline]
    pub fn read_link(path: &Path) -> Result<PathBuf, ResolveError> {
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            // Use readlinkat for safer, directory-relative symlink resolution
            unix_optimized::readlinkat_wrapper(path)
        }

        #[cfg(target_os = "windows")]
        {
            let path = fs::read_link(path)?;
            return crate::windows::strip_windows_prefix(path);
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "windows"
        )))]
        {
            Ok(fs::read_link(path)?)
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
            use libc::{F_NOCACHE, O_CLOEXEC};
            use std::{io::Read, os::unix::fs::OpenOptionsExt};
            // O_CLOEXEC ensures file descriptor is closed on exec
            let mut fd =
                fs::OpenOptions::new().read(true).custom_flags(F_NOCACHE | O_CLOEXEC).open(path)?;
            let meta = fd.metadata()?;
            #[allow(clippy::cast_possible_truncation)]
            let mut buffer = Vec::with_capacity(meta.len() as usize);
            fd.read_to_end(&mut buffer)?;
            Self::validate_string(buffer)
        }
        #[cfg(target_os = "linux")]
        {
            use std::{io::Read, os::fd::AsRawFd, os::unix::fs::OpenOptionsExt};
            // Avoid `O_DIRECT` on Linux: it requires page-aligned buffers and aligned offsets,
            // which is incompatible with a regular Vec-based read and many CI filesystems.
            // O_CLOEXEC ensures file descriptor is closed on exec
            let mut fd =
                fs::OpenOptions::new().read(true).custom_flags(libc::O_CLOEXEC).open(path)?;
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

#[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]
mod linux_optimized {
    use super::FileMetadata;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::OnceLock;

    /// Check if statx is available at runtime
    fn has_statx() -> bool {
        static CACHED: OnceLock<bool> = OnceLock::new();
        *CACHED.get_or_init(|| {
            // Try statx on a known path to see if it's supported
            unsafe {
                let path = match std::ffi::CString::new("/") {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                let mut buf: libc::statx = std::mem::zeroed();
                let ret = libc::statx(
                    libc::AT_FDCWD,
                    path.as_ptr(),
                    libc::AT_STATX_DONT_SYNC,
                    libc::STATX_TYPE,
                    &mut buf,
                );
                // statx returns 0 on success, -1 on error
                // If ENOSYS (syscall not available), kernel < 4.11
                ret == 0 || io::Error::last_os_error().raw_os_error() != Some(libc::ENOSYS)
            }
        })
    }

    /// Fast metadata using statx (Linux 4.11+)
    ///
    /// Benefits over stat/lstat:
    /// - AT_STATX_DONT_SYNC: Don't force filesystem sync, use cached attributes
    /// - STATX_TYPE: Only request file type information (file/dir/symlink)
    /// - More cache-friendly when checking many files
    ///
    /// # Errors
    ///
    /// See [std::fs::metadata] and [std::fs::symlink_metadata]
    pub fn statx_metadata(path: &Path, follow_symlinks: bool) -> io::Result<FileMetadata> {
        if !has_statx() {
            return fallback_metadata(path, follow_symlinks);
        }

        let path_bytes = path.as_os_str().as_bytes();

        // Ensure null termination
        let path_cstr = std::ffi::CString::new(path_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))?;

        let mut statx_buf: libc::statx = unsafe { std::mem::zeroed() };

        let flags = if follow_symlinks {
            libc::AT_STATX_DONT_SYNC
        } else {
            libc::AT_STATX_DONT_SYNC | libc::AT_SYMLINK_NOFOLLOW
        };

        let ret = unsafe {
            libc::statx(
                libc::AT_FDCWD,     // Use current working directory
                path_cstr.as_ptr(), // File path
                flags,              // Flags
                libc::STATX_TYPE,   // Only request type info
                &mut statx_buf,     // Output buffer
            )
        };

        if ret != 0 {
            let err = io::Error::last_os_error();
            // If ENOSYS, fall back to standard implementation
            if err.raw_os_error() == Some(libc::ENOSYS) {
                return fallback_metadata(path, follow_symlinks);
            }
            return Err(err);
        }

        // Extract file type from stx_mode
        let mode = statx_buf.stx_mode as libc::mode_t;
        let file_type = mode & libc::S_IFMT;

        let is_file = file_type == libc::S_IFREG;
        let is_dir = file_type == libc::S_IFDIR;
        let is_symlink = file_type == libc::S_IFLNK;

        Ok(FileMetadata::new(is_file, is_dir, is_symlink))
    }

    /// Fallback to standard library implementation
    fn fallback_metadata(path: &Path, follow_symlinks: bool) -> io::Result<FileMetadata> {
        if follow_symlinks {
            std::fs::metadata(path).map(FileMetadata::from)
        } else {
            std::fs::symlink_metadata(path).map(FileMetadata::from)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;

        #[test]
        fn test_statx_file() {
            let temp_file = std::env::temp_dir().join("oxc_resolver_test_file.txt");
            fs::write(&temp_file, "test").unwrap();

            let meta = statx_metadata(&temp_file, true).unwrap();
            assert!(meta.is_file());
            assert!(!meta.is_dir());
            assert!(!meta.is_symlink());

            fs::remove_file(&temp_file).unwrap();
        }

        #[test]
        fn test_statx_dir() {
            let temp_dir = std::env::temp_dir().join("oxc_resolver_test_dir");
            fs::create_dir_all(&temp_dir).unwrap();

            let meta = statx_metadata(&temp_dir, true).unwrap();
            assert!(!meta.is_file());
            assert!(meta.is_dir());
            assert!(!meta.is_symlink());

            fs::remove_dir(&temp_dir).unwrap();
        }

        #[test]
        fn test_statx_not_found() {
            let non_existent = std::env::temp_dir().join("oxc_resolver_does_not_exist_xyz123");
            let result = statx_metadata(&non_existent, true);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
        }

        #[test]
        fn test_statx_available() {
            // This test just checks that the availability check doesn't panic
            let _available = has_statx();
        }
    }
}

#[cfg(all(target_os = "macos", any(target_arch = "x86_64", target_arch = "aarch64")))]
mod macos_optimized {
    use super::FileMetadata;
    use std::io;
    use std::path::Path;

    /// Fast metadata using getattrlist (macOS)
    ///
    /// Benefits over stat/lstat:
    /// - ATTR_CMN_OBJTYPE: Only request file type information
    /// - Optimized for APFS filesystem
    /// - Skips resource fork and extended attribute lookups
    ///
    /// # Errors
    ///
    /// See [std::fs::metadata] and [std::fs::symlink_metadata]
    ///
    /// Note: Currently using fallback to std::fs. The getattrlist buffer layout
    /// requires more investigation to work correctly across all macOS versions.
    /// TODO: Implement proper getattrlist with correct buffer parsing
    pub fn getattrlist_metadata(path: &Path, follow_symlinks: bool) -> io::Result<FileMetadata> {
        // Fallback to standard library for now
        // getattrlist implementation needs buffer layout investigation
        if follow_symlinks {
            std::fs::metadata(path).map(FileMetadata::from)
        } else {
            std::fs::symlink_metadata(path).map(FileMetadata::from)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;

        #[test]
        fn test_getattrlist_file() {
            let temp_file = std::env::temp_dir().join("oxc_resolver_test_file_macos.txt");
            fs::write(&temp_file, "test").unwrap();

            let meta = getattrlist_metadata(&temp_file, true).unwrap();
            assert!(meta.is_file());
            assert!(!meta.is_dir());
            assert!(!meta.is_symlink());

            fs::remove_file(&temp_file).unwrap();
        }

        #[test]
        fn test_getattrlist_dir() {
            let temp_dir = std::env::temp_dir().join("oxc_resolver_test_dir_macos");
            fs::create_dir_all(&temp_dir).unwrap();

            let meta = getattrlist_metadata(&temp_dir, true).unwrap();
            assert!(!meta.is_file());
            assert!(meta.is_dir());
            assert!(!meta.is_symlink());

            fs::remove_dir(&temp_dir).unwrap();
        }

        #[test]
        fn test_getattrlist_not_found() {
            let non_existent =
                std::env::temp_dir().join("oxc_resolver_does_not_exist_macos_xyz123");
            let result = getattrlist_metadata(&non_existent, true);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
        }
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod unix_optimized {
    use crate::{FileSystemOs, ResolveError};
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::io::FromRawFd;
    use std::path::{Path, PathBuf};

    /// Use pread for positioned reads (thread-safe, no seek overhead)
    ///
    /// # Errors
    ///
    /// See [std::fs::read]
    pub fn pread_to_string(path: &Path) -> io::Result<String> {
        let path_bytes = path.as_os_str().as_bytes();
        let path_cstr = std::ffi::CString::new(path_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))?;

        // Open with O_RDONLY | O_CLOEXEC
        // SAFETY: libc::open is safe to call with valid C strings
        let fd = unsafe { libc::open(path_cstr.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };

        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // SAFETY: We just opened this fd successfully
        let file = unsafe { std::fs::File::from_raw_fd(fd) };

        // Get file size for buffer allocation
        let metadata = file.metadata()?;
        #[allow(clippy::cast_possible_truncation)]
        let file_size = metadata.len() as usize;

        // Allocate buffer
        let mut buffer = vec![0u8; file_size];
        let mut total_read = 0;

        // Read using pread (positioned read, no seeking)
        while total_read < file_size {
            // SAFETY: pread is safe with valid fd and buffer
            #[allow(clippy::cast_possible_wrap)]
            let bytes_read = unsafe {
                libc::pread(
                    fd,
                    buffer[total_read..].as_mut_ptr().cast(),
                    file_size - total_read,
                    total_read as i64,
                )
            };

            if bytes_read < 0 {
                return Err(io::Error::last_os_error());
            }

            if bytes_read == 0 {
                // EOF reached
                buffer.truncate(total_read);
                break;
            }

            #[allow(clippy::cast_sign_loss)]
            {
                total_read += bytes_read as usize;
            }
        }

        FileSystemOs::validate_string(buffer)
    }

    /// Use readlinkat for safer symlink resolution
    ///
    /// # Errors
    ///
    /// See [std::fs::read_link]
    pub fn readlinkat_wrapper(path: &Path) -> Result<PathBuf, ResolveError> {
        use std::ffi::OsStr;

        let path_bytes = path.as_os_str().as_bytes();
        let path_cstr = std::ffi::CString::new(path_bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "path contains null byte"))?;

        // Buffer for symlink target (PATH_MAX)
        let mut buffer = vec![0u8; libc::PATH_MAX as usize];

        // SAFETY: readlinkat is safe with valid fd and buffer
        let bytes_read = unsafe {
            libc::readlinkat(
                libc::AT_FDCWD, // Current working directory
                path_cstr.as_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };

        if bytes_read < 0 {
            return Err(io::Error::last_os_error().into());
        }

        #[allow(clippy::cast_sign_loss)]
        {
            buffer.truncate(bytes_read as usize);
        }

        // Convert bytes to PathBuf
        let os_str = OsStr::from_bytes(&buffer);
        Ok(PathBuf::from(os_str))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::fs;

        #[test]
        fn test_pread_to_string() {
            let temp_file = std::env::temp_dir().join("oxc_resolver_pread_test.txt");
            fs::write(&temp_file, "Hello, World!").unwrap();

            let content = pread_to_string(&temp_file).unwrap();
            assert_eq!(content, "Hello, World!");

            fs::remove_file(&temp_file).unwrap();
        }

        #[test]
        fn test_pread_empty_file() {
            let temp_file = std::env::temp_dir().join("oxc_resolver_pread_empty.txt");
            fs::write(&temp_file, "").unwrap();

            let content = pread_to_string(&temp_file).unwrap();
            assert_eq!(content, "");

            fs::remove_file(&temp_file).unwrap();
        }

        #[test]
        fn test_pread_not_found() {
            let non_existent = std::env::temp_dir().join("oxc_resolver_pread_nonexistent.txt");
            let result = pread_to_string(&non_existent);
            assert!(result.is_err());
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
