//! The calls into libc, and nothing else: whether a folder is writable.
//! `unsafe` lives here alone (the crate is `deny(unsafe_code)`), each call over a path turned
//! into a C string first.

use std::path::Path;

/// Whether this user may create and rename entries in `dir`:
/// `access(dir, W_OK)`. A folder that does not exist is not writable, and
/// neither is one on a read-only volume.
#[cfg(unix)]
#[allow(unsafe_code)]
pub fn writable(dir: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt as _;
    let Ok(path) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `path` is a valid NUL-terminated string that lives across the
    // call; `access` reads it and keeps nothing.
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(not(unix))]
pub fn writable(_dir: &Path) -> bool {
    false
}
