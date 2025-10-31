use std::{
    io,
    path::{Path, PathBuf},
};

use rustix::{
    fs::{Mode, OFlags},
    io::Errno,
};

#[cfg(target_os = "linux")]
use rustix::fs::AtFlags;

use crate::{ResolveError, file_system::FileMetadata};

#[cfg(target_os = "linux")]
use rustix::fs::StatxFlags;

/// Retrieve metadata using rustix's statx with minimal field mask.
///
/// This uses `statx()` which is more efficient than traditional `stat()`
/// as we can request only the fields we need (file type).
///
/// Falls back to `std::fs::metadata()` if rustix fails.
///
/// # Errors
///
/// Returns an error if the file does not exist or cannot be accessed.
pub fn metadata(path: &Path) -> io::Result<FileMetadata> {
    metadata_impl(path, true)
}

/// Retrieve symlink metadata using rustix's statx with AT_SYMLINK_NOFOLLOW.
///
/// This doesn't follow symbolic links, similar to `lstat()`.
///
/// Falls back to `std::fs::symlink_metadata()` if rustix fails.
///
/// # Errors
///
/// Returns an error if the file does not exist or cannot be accessed.
pub fn symlink_metadata(path: &Path) -> io::Result<FileMetadata> {
    metadata_impl(path, false)
}

/// Internal implementation for metadata retrieval using rustix statx.
///
/// # Parameters
/// - `path`: The file path to query
/// - `follow_symlinks`: Whether to follow symbolic links (true for metadata, false for symlink_metadata)
fn metadata_impl(path: &Path, follow_symlinks: bool) -> io::Result<FileMetadata> {
    // Try rustix statx first for better performance
    if let Ok(meta) = statx_metadata(path, follow_symlinks) {
        Ok(meta)
    } else {
        // Fallback to std::fs if rustix fails
        let std_meta = if follow_symlinks {
            std::fs::metadata(path)?
        } else {
            std::fs::symlink_metadata(path)?
        };
        Ok(FileMetadata::from(std_meta))
    }
}

/// Use rustix statx to get file metadata with minimal field requests.
///
/// We only request STATX_TYPE to determine if it's a file, directory, or symlink.
/// This is more efficient than requesting all file attributes.
///
/// On Linux, uses statx(). On other Unix platforms, uses stat/lstat.
fn statx_metadata(path: &Path, follow_symlinks: bool) -> Result<FileMetadata, Errno> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{CWD, statx};

        let flags = if follow_symlinks { AtFlags::empty() } else { AtFlags::SYMLINK_NOFOLLOW };

        // Request only the file type - most efficient for our use case
        let mask = StatxFlags::TYPE;

        let stat = statx(CWD, path, flags, mask)?;

        // Determine file type from mode
        let mode = stat.stx_mode;
        let is_file = (mode & libc::S_IFMT as u32) == libc::S_IFREG as u32;
        let is_dir = (mode & libc::S_IFMT as u32) == libc::S_IFDIR as u32;
        let is_symlink = (mode & libc::S_IFMT as u32) == libc::S_IFLNK as u32;

        Ok(FileMetadata::new(is_file, is_dir, is_symlink))
    }
    #[cfg(not(target_os = "linux"))]
    {
        // On non-Linux Unix systems, use regular stat/lstat
        use rustix::fs::{lstat, stat};

        let stat_result = if follow_symlinks { stat(path)? } else { lstat(path)? };

        let mode = u32::from(stat_result.st_mode);
        let is_file = (mode & u32::from(libc::S_IFMT)) == u32::from(libc::S_IFREG);
        let is_dir = (mode & u32::from(libc::S_IFMT)) == u32::from(libc::S_IFDIR);
        let is_symlink = (mode & u32::from(libc::S_IFMT)) == u32::from(libc::S_IFLNK);

        Ok(FileMetadata::new(is_file, is_dir, is_symlink))
    }
}

/// Read symlink target using rustix readlinkat.
///
/// Falls back to `std::fs::read_link()` if rustix fails.
///
/// # Errors
///
/// Returns `ResolveError::IOError` if the link cannot be read.
pub fn read_link(path: &Path) -> Result<PathBuf, ResolveError> {
    if let Ok(target) = read_link_impl(path) {
        Ok(target)
    } else {
        // Fallback to std::fs if rustix fails
        let target = std::fs::read_link(path)?;
        Ok(target)
    }
}

/// Internal implementation using rustix readlinkat.
fn read_link_impl(path: &Path) -> Result<PathBuf, Errno> {
    use rustix::fs::{CWD, readlinkat};
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    // readlinkat in rustix 0.38 returns a CString
    // Start with a reasonable buffer size
    let buffer = vec![0u8; 1024];

    // readlinkat takes ownership of the buffer and returns a CString
    let result = readlinkat(CWD, path, buffer)?;

    // Convert CString to PathBuf
    let bytes = result.as_bytes();
    Ok(PathBuf::from(OsStr::from_bytes(bytes)))
}

/// Read file to string while bypassing system cache using rustix.
///
/// On Linux, uses O_NOATIME to avoid updating access times for better performance.
/// On macOS, uses F_NOCACHE (via libc) to hint the kernel not to cache.
///
/// Falls back to platform-specific libc implementation or std::fs if rustix fails.
///
/// # Errors
///
/// Returns an error if the file cannot be read or contains invalid UTF-8.
pub fn read_to_string_bypass_system_cache(path: &Path) -> io::Result<String> {
    read_with_cache_bypass(path).or_else(|_| read_to_string_bypass_fallback(path))
}

/// Use rustix to read file with cache bypass hints.
fn read_with_cache_bypass(path: &Path) -> Result<String, Errno> {
    use rustix::fs::{CWD, openat};
    use std::io::Read;

    // Build open flags with cache bypass hints
    // On Linux, use O_NOATIME to skip updating access time
    // This requires file ownership or CAP_FOWNER capability
    // If it fails, the fallback will use the posix_fadvise approach
    #[cfg(target_os = "linux")]
    let flags = OFlags::RDONLY | OFlags::NOATIME;

    #[cfg(not(target_os = "linux"))]
    let flags = OFlags::RDONLY;

    let fd = openat(CWD, path, flags, Mode::empty())?;

    // Read file contents
    let mut buffer = Vec::new();
    let mut file = std::fs::File::from(fd);

    file.read_to_end(&mut buffer)
        .map_err(|e| Errno::from_raw_os_error(e.raw_os_error().unwrap_or(0)))?;

    // Validate UTF-8 using the existing helper
    crate::FileSystemOs::validate_string(buffer)
        .map_err(|e| Errno::from_raw_os_error(e.raw_os_error().unwrap_or(libc::EINVAL)))
}

/// Fallback implementation using existing platform-specific code.
#[cfg(target_os = "macos")]
fn read_to_string_bypass_fallback(path: &Path) -> io::Result<String> {
    use libc::F_NOCACHE;
    use std::{fs, io::Read, os::unix::fs::OpenOptionsExt};

    let mut fd = fs::OpenOptions::new().read(true).custom_flags(F_NOCACHE).open(path)?;
    let meta = fd.metadata()?;
    #[allow(clippy::cast_possible_truncation)]
    let mut buffer = Vec::with_capacity(meta.len() as usize);
    fd.read_to_end(&mut buffer)?;
    crate::FileSystemOs::validate_string(buffer)
}

#[cfg(target_os = "linux")]
fn read_to_string_bypass_fallback(path: &Path) -> io::Result<String> {
    use std::{fs, io::Read, os::fd::AsRawFd};

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
    crate::FileSystemOs::validate_string(buffer)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn read_to_string_bypass_fallback(path: &Path) -> io::Result<String> {
    crate::FileSystemOs::read_to_string(path)
}
