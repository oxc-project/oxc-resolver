use std::{ffi::CString, io, os::unix::ffi::OsStrExt, path::Path};

use libc::{F_NOCACHE, O_RDONLY, fcntl, open};

/// macOS-specific file system optimizations
pub struct MacOsFs;

impl MacOsFs {
    /// Read file without polluting the system cache
    /// Useful for one-time reads of files that won't be accessed again
    ///
    /// # Errors
    ///
    /// * Returns any I/O error raised when opening the file, adjusting file flags, reading from
    ///   it, or determining the file metadata.
    pub fn read_nocache(path: &Path) -> io::Result<Vec<u8>> {
        let path_c = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Path contains null byte"))?;

        // SAFETY: the path_c is valid for the duration of the call.
        let fd = unsafe { open(path_c.as_ptr().cast(), O_RDONLY) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Set F_NOCACHE to avoid polluting system cache and propagate failures
        // SAFETY: `fd` is a valid open file descriptor and `F_NOCACHE` expects an integer value.
        let result = unsafe { fcntl(fd, F_NOCACHE, 1) };
        if result == -1 {
            let err = io::Error::last_os_error();
            // SAFETY: `fd` is valid and needs to be closed before returning.
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }

        // Get file size using fstat on the open fd to avoid second filesystem lookup
        // SAFETY: `stat` will be properly allocated
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        // SAFETY: `fd` is a valid file descriptor and `stat` is properly allocated
        if unsafe { libc::fstat(fd, &raw mut stat) } != 0 {
            let err = io::Error::last_os_error();
            // SAFETY: `fd` is valid and needs to be closed before returning
            unsafe {
                libc::close(fd);
            }
            return Err(err);
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let size = stat.st_size as usize;

        let mut buffer = vec![0u8; size];
        let mut total_read = 0usize;

        // Read in a loop to handle partial reads
        while total_read < size {
            // SAFETY: `fd` is valid, the buffer has enough capacity, and we're reading within bounds
            let bytes_read = unsafe {
                libc::read(
                    fd,
                    buffer.as_mut_ptr().add(total_read).cast::<libc::c_void>(),
                    size - total_read,
                )
            };

            if bytes_read < 0 {
                let err = io::Error::last_os_error();
                // SAFETY: `fd` is valid and needs to be closed before returning
                unsafe {
                    libc::close(fd);
                }
                return Err(err);
            }

            if bytes_read == 0 {
                // EOF reached
                break;
            }

            #[allow(clippy::cast_sign_loss)]
            let bytes_read = bytes_read as usize;
            total_read += bytes_read;
        }

        // SAFETY: `fd` was successfully opened earlier and must be closed exactly once.
        unsafe {
            libc::close(fd);
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_read_nocache() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_nocache.txt");

        // Create test file
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"Test data").unwrap();

        // Read without cache
        let data = MacOsFs::read_nocache(&path).unwrap();
        assert_eq!(data, b"Test data");

        // Cleanup
        fs::remove_file(&path).unwrap();
    }
}
