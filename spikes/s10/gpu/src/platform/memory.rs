//! Mach memory entries (IP14's resources): a renderer allocates a region,
//! writes pixels into it, and hands the GPU process an entry, which the GPU
//! process maps read-only. No IOKit is involved, so a renderer whose profile
//! denies `iokit-open` can share pixels this way (RV1-09).

use std::ffi::c_void;
use std::ptr::NonNull;

use super::mach::{MachError, PortName, SendRight, task};

const VM_FLAGS_ANYWHERE: i32 = 1;
const VM_PROT_READ: i32 = 1;
const VM_INHERIT_NONE: u32 = 2;

unsafe extern "C" {
    fn mach_vm_allocate(task: PortName, address: *mut u64, size: u64, flags: i32) -> i32;
    fn mach_vm_deallocate(task: PortName, address: u64, size: u64) -> i32;
    fn mach_make_memory_entry_64(
        task: PortName,
        size: *mut u64,
        offset: u64,
        permission: i32,
        handle: *mut PortName,
        parent: PortName,
    ) -> i32;
    fn mach_vm_map(
        task: PortName,
        address: *mut u64,
        size: u64,
        mask: u64,
        flags: i32,
        object: PortName,
        offset: u64,
        copy: i32,
        current: i32,
        maximum: i32,
        inheritance: u32,
    ) -> i32;
    fn getpagesize() -> i32;
}

/// The VM page size: 16 KiB on Apple silicon.
pub(crate) fn page_size() -> usize {
    // SAFETY: no arguments.
    usize::try_from(unsafe { getpagesize() }).unwrap_or(16_384)
}

/// A page-aligned region of this task's memory, deallocated on drop (G5).
#[derive(Debug)]
pub(crate) struct Region {
    address: u64,
    len: usize,
    writable: bool,
}

impl Region {
    /// A zeroed, writable region of at least `len` bytes, rounded up to whole
    /// pages.
    pub(crate) fn allocate(len: usize) -> Result<Self, MachError> {
        let len = len.max(1).next_multiple_of(page_size());
        let mut address: u64 = 0;
        // SAFETY: G2, `address`.
        let status =
            unsafe { mach_vm_allocate(task(), &raw mut address, len as u64, VM_FLAGS_ANYWHERE) };
        if status != 0 {
            return Err(MachError(status));
        }
        Ok(Self {
            address,
            len,
            writable: true,
        })
    }

    /// A memory entry for the whole region that its receiver may only read.
    pub(crate) fn entry(&self) -> Result<SendRight, MachError> {
        let mut size = self.len as u64;
        let mut handle: PortName = 0;
        // SAFETY: G2, `size` and `handle`; G5, the region is live and owned.
        let status = unsafe {
            mach_make_memory_entry_64(
                task(),
                &raw mut size,
                self.address,
                VM_PROT_READ,
                &raw mut handle,
                0,
            )
        };
        if status != 0 {
            return Err(MachError(status));
        }
        Ok(SendRight::from_raw(handle))
    }

    /// Map `len` bytes of the entry `entry` read-only into this task, shared
    /// with its writer, not copied.
    pub(crate) fn map(entry: &SendRight, len: usize) -> Result<Self, MachError> {
        let len = len.max(1).next_multiple_of(page_size());
        let mut address: u64 = 0;
        // SAFETY: G2, `address`; G3, `entry` is a live send right to a
        // memory entry.
        let status = unsafe {
            mach_vm_map(
                task(),
                &raw mut address,
                len as u64,
                0,
                VM_FLAGS_ANYWHERE,
                entry.name(),
                0,
                0,
                VM_PROT_READ,
                VM_PROT_READ,
                VM_INHERIT_NONE,
            )
        };
        if status != 0 {
            return Err(MachError(status));
        }
        Ok(Self {
            address,
            len,
            writable: false,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// The region's bytes, to write; `None` for a read-only mapping (G5).
    pub(crate) fn bytes_mut(&mut self) -> Option<&mut [u8]> {
        if !self.writable {
            return None;
        }
        let start: *mut u8 =
            std::ptr::with_exposed_provenance_mut(usize::try_from(self.address).unwrap_or(0));
        // SAFETY: G5, `len` writable bytes at `address`, owned by this value,
        // and borrowed mutably once through `&mut self`.
        Some(unsafe { std::slice::from_raw_parts_mut(start, self.len) })
    }

    /// The start, for a Metal buffer made without a copy (G7).
    pub(super) fn start(&self) -> NonNull<c_void> {
        NonNull::new(std::ptr::with_exposed_provenance_mut(
            usize::try_from(self.address).unwrap_or(0),
        ))
        .unwrap_or(NonNull::dangling())
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: G5, the region this value owns, deallocated once.
        unsafe { mach_vm_deallocate(task(), self.address, self.len as u64) };
    }
}
