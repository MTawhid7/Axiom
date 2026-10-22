//! Mach memory entries, as S10 used them (IP14's resources), and the counts
//! S16-Q1 reads: this task's VM statistics and the host's pages.

use std::ffi::c_void;
use std::ptr::NonNull;

use super::mach::{MachError, PortName, SendRight, task};

const VM_FLAGS_ANYWHERE: i32 = 1;
const VM_PROT_READ: i32 = 1;
const VM_INHERIT_NONE: u32 = 2;
/// `TASK_VM_INFO` (`mach/task_info.h`).
const TASK_VM_INFO: u32 = 22;
/// `HOST_VM_INFO64` (`mach/host_info.h`).
const HOST_VM_INFO64: i32 = 4;

/// `task_vm_info_data_t` at revision 7, packed to four bytes as the SDK's
/// Mach headers pack it: 372 bytes, `TASK_VM_INFO_COUNT` 93 (both observed
/// by compiling against the SDK, 2026-10-22).
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
#[allow(dead_code)] // the kernel fills every field; S16 reads some
struct TaskVmInfo {
    virtual_size: u64,
    region_count: i32,
    page_size: i32,
    /// From `resident_size` to `limit_bytes_remaining`, 41 eight-byte fields.
    fields: [u64; 41],
    decompressions: i32,
    tail: [i64; 3],
}

const _: () = assert!(size_of::<TaskVmInfo>() == 372);

/// Indices into [`TaskVmInfo::fields`].
mod field {
    pub(super) const RESIDENT: usize = 0;
    pub(super) const INTERNAL: usize = 4;
    pub(super) const EXTERNAL: usize = 6;
    pub(super) const PURGEABLE_VOLATILE_RESIDENT: usize = 11;
    pub(super) const COMPRESSED: usize = 13;
    pub(super) const PHYS_FOOTPRINT: usize = 16;
    pub(super) const LEDGER_PEAK: usize = 19;
    pub(super) const PURGEABLE_NONVOLATILE: usize = 20;
    pub(super) const MEDIA_FOOTPRINT: usize = 28;
    pub(super) const GRAPHICS_FOOTPRINT: usize = 32;
    pub(super) const GRAPHICS_NOFOOTPRINT: usize = 34;
}

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
    fn task_info(task: PortName, flavor: u32, info: *mut TaskVmInfo, count: *mut u32) -> i32;
    fn mach_host_self() -> PortName;
    fn host_statistics64(host: PortName, flavor: i32, info: *mut u32, count: *mut u32) -> i32;
    fn mach_vm_region_recurse(
        task: PortName,
        address: *mut u64,
        size: *mut u64,
        depth: *mut u32,
        info: *mut u32,
        count: *mut u32,
    ) -> i32;
    fn getpagesize() -> i32;
}

/// The VM page size: 16 KiB on Apple silicon.
pub(crate) fn page_size() -> usize {
    // SAFETY: no arguments.
    usize::try_from(unsafe { getpagesize() }).unwrap_or(16_384)
}

/// A page-aligned region of this task's memory, deallocated on drop (L5).
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
        // SAFETY: L2, `address`.
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
        // SAFETY: L2, `size` and `handle`; L5, the region is live and owned.
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
        // SAFETY: L2, `address`; L3, `entry` is a live send right to a
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

    /// The region's bytes, to write; `None` for a read-only mapping (L5).
    pub(crate) fn bytes_mut(&mut self) -> Option<&mut [u8]> {
        if !self.writable {
            return None;
        }
        let start: *mut u8 =
            std::ptr::with_exposed_provenance_mut(usize::try_from(self.address).unwrap_or(0));
        // SAFETY: L5, `len` writable bytes at `address`, owned by this value,
        // and borrowed mutably once through `&mut self`.
        Some(unsafe { std::slice::from_raw_parts_mut(start, self.len) })
    }

    /// The region's bytes, to read, whether or not it is writable.
    pub(crate) fn bytes(&self) -> &[u8] {
        let start: *const u8 =
            std::ptr::with_exposed_provenance(usize::try_from(self.address).unwrap_or(0));
        // SAFETY: L5, `len` readable bytes at `address`, owned by this value
        // for as long as the borrow of `self`.
        unsafe { std::slice::from_raw_parts(start, self.len) }
    }

    /// The start, for a Metal buffer made without a copy (L7).
    pub(super) fn start(&self) -> NonNull<c_void> {
        NonNull::new(std::ptr::with_exposed_provenance_mut(
            usize::try_from(self.address).unwrap_or(0),
        ))
        .unwrap_or(NonNull::dangling())
    }
}

/// What the VM system holds for the object a region maps, from
/// `mach_vm_region_recurse` (`mach/mach_vm.h`): counted in the object, not in
/// any task's page tables, so it shows memory that no task is charged for.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Residency {
    /// Bytes resident, dirtied and swapped out (compressed).
    pub(crate) resident: u64,
    pub(crate) dirtied: u64,
    pub(crate) swapped: u64,
    /// References to the object.
    pub(crate) ref_count: u32,
}

impl Region {
    /// The residency of the object this region maps. `vm_region_submap_info_64`
    /// is 84 bytes, `VM_REGION_SUBMAP_INFO_COUNT_64` 21 (observed by compiling
    /// against the SDK, 2026-10-22); its fields are read by their offsets.
    pub(crate) fn residency(&self) -> Result<Residency, MachError> {
        let mut words = [0_u32; 21];
        let (mut address, mut size, mut depth, mut count) = (self.address, 0_u64, 0_u32, 21_u32);
        // SAFETY: L2 and L10, `words` is a live array of `count` `natural_t`s,
        // the size of `vm_region_submap_info_64`; the other out-parameters are
        // live locals.
        let status = unsafe {
            mach_vm_region_recurse(
                task(),
                &raw mut address,
                &raw mut size,
                &raw mut depth,
                words.as_mut_ptr(),
                &raw mut count,
            )
        };
        if status != 0 {
            return Err(MachError(status));
        }
        if address != self.address {
            return Err(MachError(-3));
        }
        let page = page_size() as u64;
        Ok(Residency {
            resident: u64::from(words[6]) * page,
            swapped: u64::from(words[8]) * page,
            dirtied: u64::from(words[9]) * page,
            ref_count: words[10],
        })
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: L5, the region this value owns, deallocated once.
        unsafe { mach_vm_deallocate(task(), self.address, self.len as u64) };
    }
}

/// What macOS charges this task for, and where it puts it, in bytes, from
/// `TASK_VM_INFO`.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TaskVm {
    pub(crate) phys_footprint: u64,
    pub(crate) resident: u64,
    pub(crate) internal: u64,
    pub(crate) external: u64,
    pub(crate) compressed: u64,
    pub(crate) purgeable_volatile_resident: u64,
    pub(crate) purgeable_nonvolatile: u64,
    pub(crate) ledger_peak: u64,
    pub(crate) graphics_footprint: u64,
    pub(crate) graphics_nofootprint: u64,
    pub(crate) media_footprint: u64,
}

impl TaskVm {
    /// The values in a fixed order, for a message or a table.
    pub(crate) fn values(&self) -> [u64; 11] {
        [
            self.phys_footprint,
            self.resident,
            self.internal,
            self.external,
            self.compressed,
            self.purgeable_volatile_resident,
            self.purgeable_nonvolatile,
            self.ledger_peak,
            self.graphics_footprint,
            self.graphics_nofootprint,
            self.media_footprint,
        ]
    }

    pub(crate) fn from_values(v: &[u64]) -> Option<Self> {
        let get = |i: usize| v.get(i).copied();
        Some(Self {
            phys_footprint: get(0)?,
            resident: get(1)?,
            internal: get(2)?,
            external: get(3)?,
            compressed: get(4)?,
            purgeable_volatile_resident: get(5)?,
            purgeable_nonvolatile: get(6)?,
            ledger_peak: get(7)?,
            graphics_footprint: get(8)?,
            graphics_nofootprint: get(9)?,
            media_footprint: get(10)?,
        })
    }

    pub(crate) const NAMES: [&'static str; 11] = [
        "phys_footprint",
        "resident",
        "internal",
        "external",
        "compressed",
        "purgeable_volatile_resident",
        "purgeable_nonvolatile",
        "ledger_peak",
        "graphics_footprint",
        "graphics_nofootprint",
        "media_footprint",
    ];
}

/// This task's VM statistics.
pub(crate) fn task_vm() -> Result<TaskVm, MachError> {
    let mut info = TaskVmInfo {
        virtual_size: 0,
        region_count: 0,
        page_size: 0,
        fields: [0; 41],
        decompressions: 0,
        tail: [0; 3],
    };
    let mut count = u32::try_from(size_of::<TaskVmInfo>() / 4).unwrap_or(0);
    // SAFETY: L2 and L10, `info` is a live `task_vm_info_data_t` and `count`
    // its size in `natural_t`s, as `TASK_VM_INFO_COUNT` gives it.
    let status = unsafe { task_info(task(), TASK_VM_INFO, &raw mut info, &raw mut count) };
    if status != 0 {
        return Err(MachError(status));
    }
    // Copied out of the packed struct before indexing. The ledger fields are
    // `int64_t`, never negative here, so they read the same as `u64`.
    let f = info.fields;
    Ok(TaskVm {
        phys_footprint: f[field::PHYS_FOOTPRINT],
        resident: f[field::RESIDENT],
        internal: f[field::INTERNAL],
        external: f[field::EXTERNAL],
        compressed: f[field::COMPRESSED],
        purgeable_volatile_resident: f[field::PURGEABLE_VOLATILE_RESIDENT],
        purgeable_nonvolatile: f[field::PURGEABLE_NONVOLATILE],
        ledger_peak: f[field::LEDGER_PEAK],
        graphics_footprint: f[field::GRAPHICS_FOOTPRINT],
        graphics_nofootprint: f[field::GRAPHICS_NOFOOTPRINT],
        media_footprint: f[field::MEDIA_FOOTPRINT],
    })
}

/// The host's pages, from `HOST_VM_INFO64`, in pages of [`page_size`].
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HostVm {
    pub(crate) free: u64,
    pub(crate) active: u64,
    pub(crate) inactive: u64,
    pub(crate) wired: u64,
    pub(crate) speculative: u64,
    pub(crate) purgeable: u64,
    pub(crate) compressor: u64,
    pub(crate) external: u64,
    pub(crate) internal: u64,
}

impl HostVm {
    pub(crate) const NAMES: [&'static str; 9] = [
        "free",
        "active",
        "inactive",
        "wired",
        "speculative",
        "purgeable",
        "compressor",
        "external",
        "internal",
    ];

    pub(crate) fn values(&self) -> [u64; 9] {
        [
            self.free,
            self.active,
            self.inactive,
            self.wired,
            self.speculative,
            self.purgeable,
            self.compressor,
            self.external,
            self.internal,
        ]
    }
}

/// The host's page counts. `vm_statistics64_data_t` is 416 bytes,
/// `HOST_VM_INFO64_COUNT` 104 (observed, 2026-10-22); its 32-bit counts are
/// read by their byte offsets.
pub(crate) fn host_vm() -> Result<HostVm, MachError> {
    let mut words = [0_u32; 104];
    let mut count: u32 = 104;
    // SAFETY: L6, the host port, released below.
    let host = unsafe { mach_host_self() };
    // SAFETY: L2 and L10, `words` is a live array of `count` `natural_t`s,
    // the size of `vm_statistics64_data_t`.
    let status =
        unsafe { host_statistics64(host, HOST_VM_INFO64, words.as_mut_ptr(), &raw mut count) };
    drop(SendRight::from_raw(host));
    if status != 0 {
        return Err(MachError(status));
    }
    let at = |offset: usize| u64::from(words[offset / 4]);
    Ok(HostVm {
        free: at(0),
        active: at(4),
        inactive: at(8),
        wired: at(12),
        purgeable: at(88),
        speculative: at(92),
        compressor: at(128),
        external: at(136),
        internal: at(140),
    })
}
