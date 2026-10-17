//! Can this process create, fill and run JIT code? (S4-Q4)
//!
//! Following Apple's porting guide: map one region with `MAP_JIT`, switch the
//! thread to writing with `pthread_jit_write_protect_np(0)`, copy the code in,
//! switch back to executing, flush the instruction cache with
//! `sys_icache_invalidate`, then call it. The code is two fixed AArch64
//! instructions that return 42.

use core::ffi::{c_int, c_void};
use core::ptr;
use std::io;

// <sys/mman.h>
const PROT_READ: c_int = 0x01;
const PROT_WRITE: c_int = 0x02;
const PROT_EXEC: c_int = 0x04;
const MAP_PRIVATE: c_int = 0x0002;
const MAP_JIT: c_int = 0x0800;
const MAP_ANON: c_int = 0x1000;
/// One 16 KiB page on Apple silicon.
const REGION: usize = 16 * 1024;
/// `mov w0, #42` and `ret`.
const CODE: [u32; 2] = [0x5280_0540, 0xd65f_03c0];

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    fn pthread_jit_write_protect_np(enabled: c_int);
    fn sys_icache_invalidate(start: *mut c_void, len: usize);
}

/// What the attempt found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JitOutcome {
    /// `Ok` if `mmap(MAP_JIT)` succeeded, else its `errno`.
    pub map: Result<(), i32>,
    /// What the generated code returned, if it ran.
    pub ran: Option<i32>,
    /// Whether a second `MAP_JIT` region could be mapped while the first
    /// existed (Apple: one region under the hardened runtime).
    pub second_map: Option<Result<(), i32>>,
}

/// A mapped `MAP_JIT` region, unmapped on drop (X9).
struct Region(*mut c_void);

impl Region {
    fn map() -> Result<Self, i32> {
        // SAFETY: X9. A fresh anonymous mapping at an address of the kernel's
        // choosing; nothing else refers to it.
        let address = unsafe {
            mmap(
                ptr::null_mut(),
                REGION,
                PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_PRIVATE | MAP_ANON | MAP_JIT,
                -1,
                0,
            )
        };
        // MAP_FAILED is (void *)-1.
        if address.addr() == usize::MAX {
            Err(io::Error::last_os_error().raw_os_error().unwrap_or(-1))
        } else {
            Ok(Self(address))
        }
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: X9. The region was mapped by `Region::map`, with this size,
        // and is unmapped once.
        unsafe { munmap(self.0, REGION) };
    }
}

/// Try to create, fill and run JIT code.
pub fn attempt() -> JitOutcome {
    let region = match Region::map() {
        Ok(region) => region,
        Err(errno) => {
            return JitOutcome {
                map: Err(errno),
                ran: None,
                second_map: None,
            };
        }
    };
    let code = region.0.cast::<u32>();
    // SAFETY: X9. Makes this thread's view of MAP_JIT regions writable and
    // not executable; nothing is executing in the region.
    unsafe { pthread_jit_write_protect_np(0) };
    // SAFETY: X9. `code` points to the start of a 16 KiB region, aligned for
    // `u32`, that this thread may now write. The slice covers its first eight
    // bytes, and no other reference to the region exists while it lives.
    let words = unsafe { core::slice::from_raw_parts_mut(code, CODE.len()) };
    words.copy_from_slice(&CODE);
    // SAFETY: X9. Back to executable and not writable, for this thread.
    unsafe { pthread_jit_write_protect_np(1) };
    // SAFETY: X9. Flushes the instruction cache for the bytes just written,
    // which lie inside the mapped region.
    unsafe { sys_icache_invalidate(region.0, size_of_val(&CODE)) };
    // SAFETY: X9. The region now holds `mov w0, #42; ret`, a complete
    // function with the C calling convention that takes nothing and returns
    // an int; the cache is flushed and the region is executable.
    let function: extern "C" fn() -> c_int = unsafe { core::mem::transmute(region.0) };
    let returned = function();
    let second_map = Some(Region::map().map(drop));
    drop(region);
    JitOutcome {
        map: Ok(()),
        ran: Some(returned),
        second_map,
    }
}
