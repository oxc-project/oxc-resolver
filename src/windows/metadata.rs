use std::{ffi::OsStr, io, path::Path};

// Some functions are copied and adapted from Rust standard library.
// License: https://github.com/rust-lang/rust/blob/1.89.0/LICENSE-MIT, https://github.com/rust-lang/rust/blob/1.89.0/LICENSE-APACHE

pub struct SymlinkMetadata {
    pub is_symlink: bool,
    pub is_dir: bool,
    pub is_file: bool,
}

/// Optimized version of [std::fs::symlink_metadata] for Windows.
///
/// [std::fs::symlink_metadata] implementation uses `GetFileInformationByHandle` on Windows, which is known to be slow [^1].
/// This function uses `GetFileAttributesExW` instead which is faster.
///
/// [^1]: https://github.com/gradle/native-platform/issues/203, https://github.com/dotnet/msbuild/issues/2052.
pub fn symlink_metadata(path: &Path) -> io::Result<SymlinkMetadata> {
    use windows::{
        Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAGS_AND_ATTRIBUTES,
            GetFileAttributesExW, GetFileExInfoStandard,
        },
        core::HSTRING,
    };

    let verbatim_path = maybe_verbatim(path)?;
    let lpfilename = HSTRING::from_wide(&verbatim_path);
    let finfolevelid = GetFileExInfoStandard;
    let mut file_info = std::mem::MaybeUninit::<
        windows::Win32::Storage::FileSystem::WIN32_FILE_ATTRIBUTE_DATA,
    >::uninit();

    // SAFETY: `lpfileinformation` is a valid pointer to a `WIN32_FILE_ATTRIBUTE_DATA` struct.
    unsafe { GetFileAttributesExW(&lpfilename, finfolevelid, (&raw mut file_info).cast()) }?;
    // SAFETY: `file_info` has been initialized by `GetFileAttributesExW`.
    let file_info = unsafe { file_info.assume_init() };

    let file_attrs = FILE_FLAGS_AND_ATTRIBUTES(file_info.dwFileAttributes);
    let is_directory = file_attrs.contains(FILE_ATTRIBUTE_DIRECTORY);
    // NOTE: this does not handle `is_reparse_tag_name_surrogate` which is handled by std lib
    // https://github.com/rust-lang/rust/blob/1.89.0/library/std/src/sys/fs/windows.rs#L1122-L1124
    let is_symlink = file_attrs.contains(FILE_ATTRIBUTE_REPARSE_POINT);
    Ok(SymlinkMetadata {
        is_dir: !is_symlink && is_directory,
        is_file: !is_symlink && !is_directory,
        is_symlink,
    })
}

/// Returns a UTF-16 encoded path capable of bypassing the legacy `MAX_PATH` limits.
///
/// This path may or may not have a verbatim prefix.
///
/// Based on <https://github.com/rust-lang/rust/blob/1.89.0/library/std/src/sys/path/windows.rs#L82-L88>
fn maybe_verbatim(path: &Path) -> io::Result<Vec<u16>> {
    let path = to_u16s(path)?;
    get_long_path(path)
}

/// Gets a normalized absolute path that can bypass path length limits.
///
/// Based on <https://github.com/rust-lang/rust/blob/1.89.0/library/std/src/sys/path/windows.rs#L90-L186> and <https://github.com/microsoft/sudo/blob/9f50d79704a9d4d468bc59f725993714762981ca/sudo/src/helpers.rs#L514>
///
/// License of sudo: <https://github.com/microsoft/sudo/blob/9f50d79704a9d4d468bc59f725993714762981ca/LICENSE>
fn get_long_path(mut path: Vec<u16>) -> io::Result<Vec<u16>> {
    use windows::Win32::Storage::FileSystem::GetFullPathNameW;
    use windows::core::HSTRING;

    // Normally the MAX_PATH is 260 UTF-16 code units (including the NULL).
    // However, for APIs such as CreateDirectory[1], the limit is 248.
    //
    // [1]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectorya#parameters
    const LEGACY_MAX_PATH: usize = 248;
    // UTF-16 encoded code points, used in parsing and building UTF-16 paths.
    // All of these are in the ASCII range so they can be cast directly to `u16`.
    const SEP: u16 = b'\\' as _;
    const ALT_SEP: u16 = b'/' as _;
    const QUERY: u16 = b'?' as _;
    const COLON: u16 = b':' as _;
    const DOT: u16 = b'.' as _;
    const U: u16 = b'U' as _;
    const N: u16 = b'N' as _;
    const C: u16 = b'C' as _;

    // \\?\
    const VERBATIM_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP];
    // \??\
    const NT_PREFIX: &[u16] = &[SEP, QUERY, QUERY, SEP];
    // \\?\UNC\
    const UNC_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP, U, N, C, SEP];

    if path.starts_with(VERBATIM_PREFIX) || path.starts_with(NT_PREFIX) || path == [0] {
        // Early return for paths that are already verbatim or empty.
        return Ok(path);
    } else if path.len() < LEGACY_MAX_PATH {
        // Early return if an absolute path is less < 260 UTF-16 code units.
        // This is an optimization to avoid calling `GetFullPathNameW` unnecessarily.
        match path.as_slice() {
            // Starts with `D:`, `D:\`, `D:/`, etc.
            // Does not match if the path starts with a `\` or `/`.
            [drive, COLON, 0] | [drive, COLON, SEP | ALT_SEP, ..]
                if *drive != SEP && *drive != ALT_SEP =>
            {
                return Ok(path);
            }
            // Starts with `\\`, `//`, etc
            [SEP | ALT_SEP, SEP | ALT_SEP, ..] => return Ok(path),
            _ => {}
        }
    }

    let lpfilename = HSTRING::from_wide(&path);
    let mut buffer = vec![0u16; LEGACY_MAX_PATH * 2];
    loop {
        // SAFETY: ???
        let res = unsafe { GetFullPathNameW(&lpfilename, Some(buffer.as_mut_slice()), None) };
        // GetFullPathNameW will return the required buffer size if the buffer is too small.
        match res as usize {
            0 => return Err(io::Error::last_os_error()),
            len if len <= buffer.len() => {
                let mut buffer = &buffer[..len];

                // Secondly, add the verbatim prefix. This is easier here because we know the
                // path is now absolute and fully normalized (e.g. `/` has been changed to `\`).
                let prefix = match buffer {
                    // C:\ => \\?\C:\
                    [_, COLON, SEP, ..] => VERBATIM_PREFIX,
                    // \\.\ => \\?\
                    [SEP, SEP, DOT, SEP, ..] => {
                        buffer = &buffer[4..];
                        VERBATIM_PREFIX
                    }
                    // Leave \\?\ and \??\ as-is.
                    [SEP, SEP | QUERY, QUERY, SEP, ..] => &[],
                    // \\ => \\?\UNC\
                    [SEP, SEP, ..] => {
                        buffer = &buffer[2..];
                        UNC_PREFIX
                    }
                    // Anything else we leave alone.
                    _ => &[],
                };

                path.clear();
                path.reserve_exact(prefix.len() + buffer.len() + 1);
                path.extend_from_slice(prefix);
                path.extend_from_slice(buffer);
                path.push(0);
                return Ok(path);
            }
            new_len => buffer.resize(new_len, 0),
        }
    }
}

/// Copied from <https://github.com/rust-lang/rust/blob/1.89.0/library/std/src/sys/pal/windows/mod.rs#L169-L188>
fn to_u16s<S: AsRef<OsStr>>(s: S) -> io::Result<Vec<u16>> {
    fn inner(s: &OsStr) -> io::Result<Vec<u16>> {
        // Most paths are ASCII, so reserve capacity for as much as there are bytes
        // in the OsStr plus one for the null-terminating character. We are not
        // wasting bytes here as paths created by this function are primarily used
        // in an ephemeral fashion.

        use std::os::windows::ffi::OsStrExt;
        let mut maybe_result = Vec::with_capacity(s.len() + 1);
        maybe_result.extend(s.encode_wide());

        if unrolled_find_u16s(0, &maybe_result).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "strings passed to WinAPI cannot contain NULs",
            ));
        }
        maybe_result.push(0);
        Ok(maybe_result)
    }
    inner(s.as_ref())
}

/// Copied from <https://github.com/rust-lang/rust/blob/1.89.0/library/std/src/sys/pal/windows/mod.rs#L140-L167>
fn unrolled_find_u16s(needle: u16, haystack: &[u16]) -> Option<usize> {
    let ptr = haystack.as_ptr();
    let mut start = haystack;

    // For performance reasons unfold the loop eight times.
    while start.len() >= 8 {
        macro_rules! if_return {
            ($($n:literal,)+) => {
                $(
                    if start[$n] == needle {
                        return Some(((&raw const start[$n]).addr() - ptr.addr()) / 2);
                    }
                )+
            }
        }

        if_return!(0, 1, 2, 3, 4, 5, 6, 7,);

        start = &start[8..];
    }

    for c in start {
        if *c == needle {
            return Some((std::ptr::from_ref::<u16>(c).addr() - ptr.addr()) / 2);
        }
    }
    None
}
