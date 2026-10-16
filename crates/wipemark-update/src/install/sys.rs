//! The calls into libc, and nothing else: whether a folder is writable, and
//! on macOS the one atomic swap of two directories.
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

/// Swap `a` and `b` in one call: `renamex_np(a, b, RENAME_SWAP)`, atomic on
/// APFS — after it, each name holds what the other did, and no moment
/// exists in which either is missing.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn swap(a: &Path, b: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;
    let c = |path: &Path| {
        std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))
    };
    let (from, to) = (c(a)?, c(b)?);
    // SAFETY: both are valid NUL-terminated strings that live across the
    // call; `renamex_np` reads them and keeps nothing.
    let answer = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_SWAP) };
    if answer == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
