//! IOSurface objects: the tiles' backing stores. Each is drawn by Metal through a
//! texture that shares its memory, and shown as a layer's contents (GR2). Its
//! colour space travels with it as a property (GR6).

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceDisplayP3, kCGColorSpaceSRGB};
use objc2_io_surface::{
    IOSurfaceRef, kIOSurfaceBytesPerElement, kIOSurfaceColorSpace, kIOSurfaceHeight,
    kIOSurfacePixelFormat, kIOSurfaceWidth,
};
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};

/// `'BGRA'`, IOSurface's name for 8-bit BGRA (the `kCVPixelFormatType_32BGRA`
/// four-character code).
const PIXEL_FORMAT_BGRA: i32 = i32::from_be_bytes(*b"BGRA");
/// The largest surface side the compositor creates; Metal's limit is 16,384.
pub(crate) const MAX_SIDE: usize = 8192;

/// The colour space a surface is tagged with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tag {
    DisplayP3,
    Srgb,
    /// No tag: CoreAnimation decides.
    None,
}

/// An IOSurface and the Metal texture that draws into it.
#[derive(Debug)]
pub(crate) struct Surface {
    surface: CFRetained<IOSurfaceRef>,
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
}

impl Surface {
    /// A `width` × `height` BGRA surface, tagged `tag`, with a render target
    /// texture on `device`.
    pub(crate) fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        width: usize,
        height: usize,
        tag: Tag,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return Err(format!("a {width}×{height} surface is out of range"));
        }
        let number = |n: usize| CFNumber::new_i64(i64::try_from(n).unwrap_or(i64::MAX));
        let (w, h, bpe) = (number(width), number(height), number(4));
        let format = CFNumber::new_i32(PIXEL_FORMAT_BGRA);
        // SAFETY: K6.
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
        // SAFETY: K8. The dictionary maps IOSurface's keys to CFNumbers.
        let surface =
            unsafe { IOSurfaceRef::new(properties.as_opaque()) }.ok_or("IOSurfaceCreate failed")?;
        if let Some(space) = color_space(tag) {
            let list = space
                .property_list()
                .ok_or("no colour space property list")?;
            // SAFETY: K6.
            let key = unsafe { kIOSurfaceColorSpace };
            // SAFETY: K8. The colour space key takes a colour space's property
            // list.
            unsafe { surface.set_value(key, &list) };
        }

        // SAFETY: the size is non-zero and within Metal's 16,384-pixel limit
        // (checked above), and the surface is BGRA8, matching the format.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::BGRA8Unorm,
                width,
                height,
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

    /// Whether the window server still uses the surface. A surface in use is
    /// not drawn into, as Firefox's surface pool checks.
    pub(crate) fn in_use(&self) -> bool {
        self.surface.is_in_use()
    }

    /// Whether the surface carries a colour space tag.
    pub(crate) fn tagged(&self) -> bool {
        // SAFETY: K6.
        let key = unsafe { kIOSurfaceColorSpace };
        self.surface.value(key).is_some()
    }

    /// The bytes IOSurface allocated for it.
    pub(crate) fn bytes(&self) -> usize {
        self.surface.alloc_size()
    }

    /// The surface as an object, for a layer's contents.
    pub(crate) fn as_object(&self) -> &AnyObject {
        let cf: &CFType = (*self.surface).as_ref();
        cf.as_ref()
    }
}

fn color_space(tag: Tag) -> Option<CFRetained<CGColorSpace>> {
    let name = match tag {
        // SAFETY: K6.
        Tag::DisplayP3 => unsafe { kCGColorSpaceDisplayP3 },
        // SAFETY: K6.
        Tag::Srgb => unsafe { kCGColorSpaceSRGB },
        Tag::None => return None,
    };
    CGColorSpace::with_name(Some(name))
}
