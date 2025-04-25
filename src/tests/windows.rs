#[cfg(target_os = "windows")]
use std::{
    ffi::{OsStr, OsString},
    fs::canonicalize,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
};

use thiserror::Error;

/// Converts a Win32 drive letter or mounted folder into DOS device path, e.g.:
/// `\\?\Volume{GUID}\`
#[cfg(target_os = "windows")]
pub fn volume_name_from_mount_point<S: AsRef<OsStr>>(
    mount_point: S,
) -> Result<OsString, Win32Error> {
    use windows_sys::Win32::{
        Foundation::GetLastError, Storage::FileSystem::GetVolumeNameForVolumeMountPointW,
    };

    const BUFFER_SIZE: u32 = 64;
    let mount_point: Vec<u16> = mount_point.as_ref().encode_wide().chain(Some(0)).collect();
    // A reasonable size for the buffer to accommodate the largest possible volume GUID path is 50 characters.
    let mut buffer = vec![0; BUFFER_SIZE as usize];
    // SAFETY: Win32 API call
    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumenameforvolumemountpointw
        if GetVolumeNameForVolumeMountPointW(mount_point.as_ptr(), buffer.as_mut_ptr(), BUFFER_SIZE)
            == 0
        {
            Err(Win32Error { error_code: GetLastError() })
        } else {
            let length = buffer.iter().position(|&c| c == 0).unwrap();
            Ok(OsString::from_wide(&buffer[..length]))
        }
    }
}

#[cfg(target_os = "windows")]
pub fn get_dos_device_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, Win32Error> {
    let path = path.as_ref();
    assert!(path.has_root(), "Expected a path with a root");

    let root = {
        // lpszVolumeMountPoint: The string must end with a trailing backslash ('\').
        let mut root = OsString::from(path.components().next().unwrap().as_os_str());
        root.push(r"\");
        root
    };
    let mut volume_name_root: Vec<u16> =
        volume_name_from_mount_point(root)?.encode_wide().collect();
    if volume_name_root.starts_with(&[92, 92, 63, 92] /* \\?\ */) {
        // Replace \\?\ with \\.\
        // While both is a valid DOS device path, "\\?\" won't be accepted by most of the IO operations.
        volume_name_root[2] = u16::from(b'.');
    }

    let mut dos_device_path = PathBuf::from(OsString::from_wide(&volume_name_root));
    dos_device_path.extend(path.components().skip(1));

    Ok(dos_device_path)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Win32 Error (GetLastError: {error_code:#X})")]
pub struct Win32Error {
    /// https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes
    pub error_code: u32,
}

#[cfg(target_os = "windows")]
#[test]
fn test_get_dos_device_path() {
    let root = super::fixture_root();
    println!("Fixture root: {root:?}");
    let dos_device_path = get_dos_device_path(&root).unwrap();
    println!("Dos device path: {dos_device_path:?}");

    // On Windows, canonicalize will resolve any path (traditional or DOS device path)
    // into a DOS device path with drive letter (e.g., r"\\?\D:\foo\bar.js").
    // https://doc.rust-lang.org/std/fs/fn.canonicalize.html
    let canonical_dos_device_path = canonicalize(&dos_device_path).unwrap();
    println!("-> Canonicalized: {canonical_dos_device_path:?}");

    // So eventually, the canonicalized path should be exactly the same.
    assert_eq!(canonical_dos_device_path, canonicalize(root).unwrap());
}
