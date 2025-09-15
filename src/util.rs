/// Replace UTF-8 BOM (Byte Order Mark) with whitespace to avoid allocation.
///
/// The UTF-8 BOM is the three-byte sequence 0xEF 0xBB 0xBF at the beginning of a file.
/// This function replaces these bytes with spaces in-place to avoid allocating a new string.
///
/// # Safety
/// This function uses unsafe code to get mutable access to the string's bytes.
/// This is safe because:
/// - We only replace valid UTF-8 bytes (BOM) with valid UTF-8 bytes (spaces)
/// - Spaces are single-byte ASCII characters that are valid UTF-8
pub fn replace_bom_with_whitespace(json: &mut String) {
    if json.len() >= 3 {
        let bytes = unsafe { json.as_bytes_mut() };
        if bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            bytes[0] = b' ';
            bytes[1] = b' ';
            bytes[2] = b' ';
        }
    }
}
