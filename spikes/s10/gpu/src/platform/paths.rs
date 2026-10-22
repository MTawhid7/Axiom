//! Per-user directories the GPU process's profile names.

use std::ffi::c_char;
use std::path::PathBuf;

/// `_CS_DARWIN_USER_CACHE_DIR`, from `unistd.h`.
const DARWIN_USER_CACHE_DIR: i32 = 65_538;

unsafe extern "C" {
    fn confstr(name: i32, buffer: *mut c_char, length: usize) -> usize;
}

/// This user's cache directory, where Metal keeps its shader caches.
pub(crate) fn user_cache_dir() -> Option<PathBuf> {
    let mut buffer = vec![0 as c_char; 1024];
    // SAFETY: G2, `buffer` is a live array of the length passed.
    let length = unsafe { confstr(DARWIN_USER_CACHE_DIR, buffer.as_mut_ptr(), buffer.len()) };
    if length == 0 || length > buffer.len() {
        return None;
    }
    let bytes: Vec<u8> = buffer[..length - 1]
        .iter()
        .map(|c| c.to_ne_bytes()[0])
        .collect();
    String::from_utf8(bytes).ok().map(PathBuf::from)
}
