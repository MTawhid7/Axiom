//! Read-only mappings of system font files (TX6, TX21: "system fonts are
//! mapped files, shared between processes"). Pages are read only when the
//! shaper or rasterizer touches them, and as clean file-backed pages they don't
//! count in a process's footprint.

use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::ptr::NonNull;

// <sys/mman.h>, SDK 27.0.
const PROT_READ: i32 = 0x01;
const MAP_PRIVATE: i32 = 0x0002;
const MAP_FAILED: *mut c_void = std::ptr::without_provenance_mut(usize::MAX);

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
}

/// A whole file, mapped read-only.
#[derive(Debug)]
pub(crate) struct Mapping {
    ptr: NonNull<u8>,
    len: usize,
}

// SAFETY: C5. The mapping is immutable memory owned by this value alone, so
// sharing references to it, or moving it, between threads is sound.
unsafe impl Send for Mapping {}
// SAFETY: C5, as above.
unsafe impl Sync for Mapping {}

impl Mapping {
    /// Map `path` read-only. An empty file maps to an empty slice.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = usize::try_from(file.metadata()?.len()).map_err(io::Error::other)?;
        if len == 0 {
            return Ok(Self {
                ptr: NonNull::dangling(),
                len: 0,
            });
        }
        // SAFETY: C5. A fresh read-only private mapping of the whole open file;
        // the descriptor may be closed afterwards (mmap(2)).
        let ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ,
                MAP_PRIVATE,
                file.as_raw_fd(),
                0,
            )
        };
        if ptr == MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let ptr = NonNull::new(ptr.cast()).ok_or_else(|| io::Error::other("mmap returned null"))?;
        Ok(Self { ptr, len })
    }
}

impl AsRef<[u8]> for Mapping {
    fn as_ref(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        // SAFETY: C5. `ptr` is the start of a live mapping of `len` readable
        // bytes, which lives as long as `self`, which the slice borrows.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        if self.len > 0 {
            // SAFETY: C5. Unmapped once, by its only owner; no slice of it
            // outlives `self`.
            unsafe { munmap(self.ptr.as_ptr().cast(), self.len) };
        }
    }
}
