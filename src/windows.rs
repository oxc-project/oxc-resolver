use std::path::PathBuf;

use crate::ResolveError;

/// When applicable, converts a [DOS device path](https://learn.microsoft.com/en-us/dotnet/standard/io/file-path-formats#dos-device-paths)
/// to a normal path (usually, "Traditional DOS paths" or "UNC path") that can be consumed by the `import`/`require` syntax of Node.js.
///
/// # Errors
/// Returns error of [`ResolveError::PathNotSupported`] kind if the path cannot be represented as a normal path.
pub fn strip_windows_prefix(path: PathBuf) -> Result<PathBuf, ResolveError> {
    // See https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
    let path_bytes = path.as_os_str().as_encoded_bytes();

    let path = if let Some(p) =
        path_bytes.strip_prefix(br"\\?\UNC\").or_else(|| path_bytes.strip_prefix(br"\\.\UNC\"))
    {
        // UNC paths
        // SAFETY: `as_encoded_bytes` ensures `p` is valid path bytes
        unsafe {
            PathBuf::from(std::ffi::OsStr::from_encoded_bytes_unchecked(&[br"\\", p].concat()))
        }
    } else if let Some(p) =
        path_bytes.strip_prefix(br"\\?\").or_else(|| path_bytes.strip_prefix(br"\\.\"))
    {
        // Assuming traditional DOS path "\\?\C:\"
        if p[1] != b':' {
            // E.g.,
            // \\?\Volume{b75e2c83-0000-0000-0000-602f00000000}
            // \\?\BootPartition\
            // It seems nodejs does not support DOS device paths with Volume GUIDs.
            // This can happen if the path points to a Mounted Volume without a drive letter.
            return Err(ResolveError::PathNotSupported(path));
        }
        // SAFETY: `as_encoded_bytes` ensures `p` is valid path bytes
        unsafe { PathBuf::from(std::ffi::OsStr::from_encoded_bytes_unchecked(p)) }
    } else {
        path
    };

    Ok(path)
}

#[test]
fn test_try_strip_windows_prefix() {
    let pass = [
        (r"C:\Users\user\Documents\", r"C:\Users\user\Documents\"),
        (r"C:\Users\user\Documents\file1.txt", r"C:\Users\user\Documents\file1.txt"),
        (r"\\?\C:\Users\user\Documents\", r"C:\Users\user\Documents\"),
        (r"\\?\C:\Users\user\Documents\file1.txt", r"C:\Users\user\Documents\file1.txt"),
        (r"\\.\C:\Users\user\Documents\file2.txt", r"C:\Users\user\Documents\file2.txt"),
        (r"\\?\UNC\server\share\file3.txt", r"\\server\share\file3.txt"),
    ];

    for (path, expected) in pass {
        assert_eq!(strip_windows_prefix(PathBuf::from(path)), Ok(PathBuf::from(expected)));
    }

    let fail = [
        r"\\?\Volume{c8ec34d8-3ba6-45c3-9b9d-3e4148e12d00}\file4.txt",
        r"\\?\BootPartition\file4.txt",
    ];

    for path in fail {
        assert_eq!(
            strip_windows_prefix(PathBuf::from(path)),
            Err(crate::ResolveError::PathNotSupported(PathBuf::from(path)))
        );
    }
}
