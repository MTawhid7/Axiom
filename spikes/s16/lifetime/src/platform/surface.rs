//! IOSurface objects: the GPU process's tiles, sent to the browser as Mach
//! send rights and shown there as layer contents (GR14), as S10 drew them;
//! with what S16 asks of them: the values a surface's creator can attach,
//! which its receiver can read (S16-Q2), its use count, and lookups of names
//! that aren't surfaces.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_foundation::{CFData, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceHeight, kIOSurfaceICCProfile,
    kIOSurfacePixelFormat, kIOSurfaceWidth,
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
}

/// A new BGRA IOSurface of `side` × `side`.
fn create(side: usize) -> Option<CFRetained<IOSurfaceRef>> {
    let number = |n: usize| CFNumber::new_i64(i64::try_from(n).unwrap_or(i64::MAX));
    let (w, h, bpe) = (number(side), number(side), number(4));
    let format = CFNumber::new_i32(PIXEL_FORMAT_BGRA);
    // SAFETY: L6, IOSurface's property keys.
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
        let surface = create(side).ok_or("IOSurfaceCreate failed")?;
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

    /// A send right to the surface, for the browser (L8).
    pub(crate) fn port(&self) -> SendRight {
        // SAFETY: L8, the call returns a new send right that the caller owns.
        SendRight::from_raw(unsafe { IOSurfaceCreateMachPort(&self.surface) })
    }

    /// Attach `bytes` under the key `key`, as any creator may.
    pub(crate) fn attach(&self, key: &str, bytes: &[u8]) {
        let (key, data) = (CFString::from_str(key), CFData::from_bytes(bytes));
        let value: &CFType = data.as_ref();
        // SAFETY: the value is a property-list type, as IOSurface's values
        // must be.
        unsafe { self.surface.set_value(&key, value) };
    }

    /// Attach `bytes` as the surface's ICC profile, whatever they hold.
    pub(crate) fn attach_icc(&self, bytes: &[u8]) {
        let data = CFData::from_bytes(bytes);
        let value: &CFType = data.as_ref();
        // SAFETY: L6, IOSurface's key; the value is a property-list type.
        unsafe { self.surface.set_value(kIOSurfaceICCProfile, value) };
    }
}

/// A surface without a texture, for S16-Q2's lookups.
#[derive(Debug)]
pub(crate) struct Owned(CFRetained<IOSurfaceRef>);

impl Owned {
    pub(crate) fn new(side: usize) -> Option<Self> {
        create(side).map(Self)
    }

    /// A send right to the surface (L8).
    pub(crate) fn port(&self) -> SendRight {
        // SAFETY: L8, the call returns a new send right that the caller owns.
        SendRight::from_raw(unsafe { IOSurfaceCreateMachPort(&self.0) })
    }
}

/// A surface found from a Mach name, as the browser finds the GPU process's
/// tiles.
#[derive(Debug)]
pub(crate) struct Received(CFRetained<IOSurfaceRef>);

impl Received {
    /// The surface `name` names, if it names one (L8): the name is the
    /// caller's still, whatever it is, even null or dead.
    pub(crate) fn look_up(name: PortName) -> Option<Self> {
        // SAFETY: L8, the lookup doesn't consume the name, and returns a
        // retained surface or nothing.
        let surface = unsafe { IOSurfaceLookupFromMachPort(name) }?;
        // SAFETY: the surface was returned retained, so it is adopted once.
        Some(Self(unsafe { CFRetained::from_raw(surface) }))
    }

    /// The surface as an object, for a layer's contents.
    pub(crate) fn as_object(&self) -> &AnyObject {
        let cf: &CFType = (*self.0).as_ref();
        cf.as_ref()
    }

    pub(crate) fn id(&self) -> u32 {
        self.0.id()
    }

    pub(crate) fn alloc_size(&self) -> usize {
        self.0.alloc_size()
    }

    pub(crate) fn is_in_use(&self) -> bool {
        self.0.is_in_use()
    }

    pub(crate) fn use_count(&self) -> i32 {
        self.0.use_count()
    }

    /// Remove every value attached to the surface, as a receiver that won't
    /// trust its creator's might.
    pub(crate) fn strip(&self) {
        self.0.remove_all_values();
    }

    /// How many values the surface carries, and the length of the value
    /// under `key` and of its ICC profile, if each is data.
    pub(crate) fn values(&self, key: &str) -> (isize, Option<usize>, Option<usize>) {
        let count = self.0.all_values().map_or(0, |all| all.count());
        let length = |value: Option<CFRetained<CFType>>| {
            value.and_then(|v| v.downcast_ref::<CFData>().map(CFData::len))
        };
        let custom = length(self.0.value(&CFString::from_str(key)));
        // SAFETY: L6, IOSurface's key.
        let icc = length(self.0.value(unsafe { kIOSurfaceICCProfile }));
        (count, custom, icc)
    }
}
