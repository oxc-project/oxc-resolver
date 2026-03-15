use std::borrow::Cow;
use std::path::PathBuf;

use crate::ResolveError;

/// Convert a `file://` URL specifier to a file path, or return the specifier as-is if it's not
/// a `file://` URL. Follows the Node.js `getPathFromURLPosix` / `getPathFromURLWin32` spec.
pub fn resolve_file_protocol(specifier: &str) -> Result<Cow<'_, str>, ResolveError> {
    if !specifier.starts_with("file://") {
        return Ok(Cow::Borrowed(specifier));
    }

    let after_scheme = &specifier["file://".len()..];

    // Split off query and fragment
    let (path_with_host, query_fragment) = after_scheme
        .find(['?', '#'])
        .map_or((after_scheme, ""), |i| (&after_scheme[..i], &after_scheme[i..]));

    // Extract hostname and pathname
    // file:///path → hostname="" pathname="/path"
    // file://host/path → hostname="host" pathname="/path"
    let (hostname, pathname) = path_with_host.strip_prefix('/').map_or_else(
        || {
            // file://host/... → hostname is everything before first /
            path_with_host
                .find('/')
                .map_or((path_with_host, ""), |i| (&path_with_host[..i], &path_with_host[i + 1..]))
        },
        |rest| ("", rest),
    );

    file_url_to_path(specifier, hostname, pathname, query_fragment)
}

/// Check if pathname contains a percent-encoded forbidden character.
/// Returns true if `%2F` (encoded `/`) is found, or on Windows also `%5C` (encoded `\`).
fn has_encoded_separators(pathname: &str) -> bool {
    let bytes = pathname.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'%'
            && ((bytes[i + 1] == b'2' && (bytes[i + 2] == b'F' || bytes[i + 2] == b'f'))
                || (cfg!(windows)
                    && bytes[i + 1] == b'5'
                    && (bytes[i + 2] == b'C' || bytes[i + 2] == b'c')))
        {
            return true;
        }
        i += 1;
    }
    false
}

fn decode_pathname<'a>(pathname: &'a str, specifier: &str) -> Result<Cow<'a, str>, ResolveError> {
    percent_encoding::percent_decode_str(pathname)
        .decode_utf8()
        .map_err(|_| ResolveError::PathNotSupported(PathBuf::from(specifier)))
}

#[cfg(not(windows))]
fn file_url_to_path(
    specifier: &str,
    hostname: &str,
    pathname: &str,
    query_fragment: &str,
) -> Result<Cow<'static, str>, ResolveError> {
    // POSIX: reject non-empty hostname
    if !hostname.is_empty() {
        return Err(ResolveError::PathNotSupported(PathBuf::from(specifier)));
    }

    if has_encoded_separators(pathname) {
        return Err(ResolveError::PathNotSupported(PathBuf::from(specifier)));
    }

    let decoded = decode_pathname(pathname, specifier)?;

    let mut result = String::with_capacity(1 + decoded.len() + query_fragment.len());
    result.push('/');
    result.push_str(&decoded);
    result.push_str(query_fragment);
    Ok(Cow::Owned(result))
}

#[cfg(windows)]
fn file_url_to_path(
    specifier: &str,
    hostname: &str,
    pathname: &str,
    query_fragment: &str,
) -> Result<Cow<'static, str>, ResolveError> {
    if has_encoded_separators(pathname) {
        return Err(ResolveError::PathNotSupported(PathBuf::from(specifier)));
    }

    let decoded = decode_pathname(pathname, specifier)?;
    let decoded = decoded.replace('/', "\\");

    let mut result = if !hostname.is_empty() {
        // UNC path
        format!("\\\\{hostname}\\{decoded}")
    } else {
        // Strip leading backslash, validate drive letter
        let path = decoded.strip_prefix('\\').unwrap_or(&decoded);
        let bytes = path.as_bytes();
        if bytes.len() < 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
            return Err(ResolveError::PathNotSupported(PathBuf::from(specifier)));
        }
        path.to_string()
    };

    result.push_str(query_fragment);
    Ok(Cow::Owned(result))
}
