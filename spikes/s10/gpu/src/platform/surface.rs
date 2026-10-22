//! IOSurface objects: the tiles the GPU process renders, sent to the browser as Mach
//! send rights and shown there as layer contents (GR14, as amended), as S6
//! drew them in one process.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceHeight, kIOSurfacePixelFormat,
    kIOSurfaceWidth,
};
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};

use super::mach::{PortName, SendRight};

/// `'BGRA'`, 8-bit BGRA (`kCVPixelFormatType_32BGRA`).
const PIXEL_FORMAT_BGRA: i32 = i32::from_be_bytes(*b"BGRA");

#[link(name = "IOSurface", kind = "framework")]
unsafe extern "C" {
    fn IOSurfaceCreateMachPort(buffer: &IOSurfaceRef) -> PortName;
    fn IOSurfaceLookupFromMachPort(port: PortName) -> Option<NonNull<IOSurfaceRef>>;
    fn IOSurfaceLock(buffer: &IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
    fn IOSurfaceUnlock(buffer: &IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
}

/// A new BGRA IOSurface of `width` × `height`.
pub(crate) fn create(width: usize, height: usize) -> Option<CFRetained<IOSurfaceRef>> {
    let number = |n: usize| CFNumber::new_i64(i64::try_from(n).unwrap_or(i64::MAX));
    let (w, h, bpe) = (number(width), number(height), number(4));
    let format = CFNumber::new_i32(PIXEL_FORMAT_BGRA);
    // SAFETY: G6, IOSurface's property keys.
    let keys: [&CFString; 4] = unsafe {
        [
            kIOSurfaceWidth,
            kIOSurfaceHeight,
            kIOSurfaceBytesPerElement,
            kIOSurfacePixelFormat,
        ]
    };
    let values: [&CFType; 4] = [&w, &h, &bpe, &format];
    let properties = CFDictionary::from_slices(&keys, &values);
    // SAFETY: the dictionary maps IOSurface's keys to CFNumbers, as
    // `IOSurfaceCreate` requires.
    unsafe { IOSurfaceRef::new(properties.as_opaque()) }
}

/// A tile: an IOSurface and the texture Metal renders into it.
#[derive(Debug)]
pub(crate) struct Tile {
    surface: CFRetained<IOSurfaceRef>,
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
}

impl Tile {
    pub(crate) fn new(device: &ProtocolObject<dyn MTLDevice>, side: usize) -> Result<Self, String> {
        let surface = create(side, side).ok_or("IOSurfaceCreate failed")?;
        // SAFETY: a non-zero, in-range BGRA8 size, matching the surface.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::BGRA8Unorm,
                side,
                side,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let texture = device
            .newTextureWithDescriptor_iosurface_plane(&descriptor, &surface, 0)
            .ok_or("no texture for the IOSurface")?;
        Ok(Self { surface, texture })
    }

    pub(crate) fn texture(&self) -> &ProtocolObject<dyn MTLTexture> {
        &self.texture
    }

    /// A send right to the surface, for the browser (G8).
    pub(crate) fn port(&self) -> SendRight {
        // SAFETY: G8, the call returns a new send right that the caller owns.
        SendRight::from_raw(unsafe { IOSurfaceCreateMachPort(&self.surface) })
    }
}

/// A renderer's own surface, filled with pixels and sent to the GPU process:
/// the path RV1-09 removes, kept for S10-Q2's comparison. Creating it needs
/// IOKit, which the renderer's profile now denies.
#[derive(Debug)]
pub(crate) struct Owned(CFRetained<IOSurfaceRef>);

impl Owned {
    pub(crate) fn new(width: usize, height: usize) -> Option<Self> {
        create(width, height).map(Self)
    }

    /// Fill every byte with `value`, as a decoder would write the pixels.
    pub(crate) fn fill(&self, value: u8) -> Result<(), String> {
        // SAFETY: G8's surface is live; a lock with no options and no seed.
        let locked = unsafe { IOSurfaceLock(&self.0, 0, std::ptr::null_mut()) };
        if locked != 0 {
            return Err(format!("IOSurfaceLock failed: {locked:#x}"));
        }
        let start = self.0.base_address().cast::<u8>();
        // SAFETY: while locked, the surface's `alloc_size` bytes at its base
        // address are this process's to write.
        unsafe { std::ptr::write_bytes(start.as_ptr(), value, self.0.alloc_size()) };
        // SAFETY: unlocks the lock taken above.
        unsafe { IOSurfaceUnlock(&self.0, 0, std::ptr::null_mut()) };
        Ok(())
    }

    /// A send right to the surface (G8).
    pub(crate) fn port(&self) -> SendRight {
        // SAFETY: G8, the call returns a new send right that the caller owns.
        SendRight::from_raw(unsafe { IOSurfaceCreateMachPort(&self.0) })
    }
}

/// A surface received as a send right: shown as layer contents by the
/// browser, or sampled by the GPU process on the old path.
#[derive(Debug)]
pub(crate) struct Received(CFRetained<IOSurfaceRef>);

impl Received {
    /// The surface a send right names (G8); the right stays the caller's.
    pub(crate) fn look_up(port: &SendRight) -> Option<Self> {
        // SAFETY: G8, the lookup doesn't consume the right, and returns a
        // retained surface or nothing.
        let surface = unsafe { IOSurfaceLookupFromMachPort(port.name()) }?;
        // SAFETY: the surface was returned retained, so it is adopted once.
        Some(Self(unsafe { CFRetained::from_raw(surface) }))
    }

    /// The surface as an object, for a layer's contents.
    pub(crate) fn as_object(&self) -> &AnyObject {
        let cf: &CFType = (*self.0).as_ref();
        cf.as_ref()
    }

    pub(crate) fn into_inner(self) -> CFRetained<IOSurfaceRef> {
        self.0
    }

    pub(crate) fn width(&self) -> usize {
        self.0.width()
    }

    pub(crate) fn height(&self) -> usize {
        self.0.height()
    }
}
