//! Metal in the GPU process: a device, two small pipelines (solid quads, and
//! quads that sample an image), tile passes timed on the GPU, and the three
//! ways S10-Q2 compares of reaching a renderer's pixels: a buffer made over
//! the mapped memory entry without a copy, read as a linear texture; one blit
//! into a private texture; and the renderer's own IOSurface, the path RV1-09
//! removes.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_foundation::CFRetained;
use objc2_foundation::NSString;
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLBlendFactor, MTLBlitCommandEncoder, MTLBuffer, MTLClearColor, MTLCommandBuffer,
    MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary,
    MTLLoadAction, MTLOrigin, MTLPixelFormat, MTLPrimitiveType, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
    MTLResourceOptions, MTLSize, MTLStorageMode, MTLStoreAction, MTLTexture, MTLTextureDescriptor,
    MTLTextureUsage, MTLViewport,
};

use super::memory::Region;
use super::time;

const SHADERS: &str = r"
#include <metal_stdlib>
using namespace metal;

struct Quad {
    float4 rect;  // page pixels: x, y, width, height
    float4 color; // premultiplied
    float4 src;   // normalized texture rectangle: u, v, width, height
};

struct Frame {
    float2 origin; // the tile's top left, in page pixels
    float2 size;   // the tile's size, in pixels
};

struct Varyings {
    float4 position [[position]];
    float4 color;
    float2 uv;
};

vertex Varyings vs_quad(uint vertex_id [[vertex_id]], uint instance [[instance_id]],
                        constant Frame& frame [[buffer(0)]],
                        const device Quad* quads [[buffer(1)]]) {
    Quad q = quads[instance];
    float2 corner = float2(float(vertex_id & 1), float(vertex_id >> 1));
    float2 p = q.rect.xy + corner * q.rect.zw - frame.origin;
    Varyings out;
    out.position = float4(p / frame.size * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    out.color = q.color;
    out.uv = q.src.xy + corner * q.src.zw;
    return out;
}

fragment float4 fs_solid(Varyings in [[stage_in]]) {
    return in.color;
}

fragment float4 fs_image(Varyings in [[stage_in]], texture2d<float> image [[texture(0)]]) {
    constexpr sampler linear_sampler(coord::normalized, filter::linear, address::clamp_to_edge);
    return image.sample(linear_sampler, in.uv);
}
";

/// One quad, as the shaders read it: 48 bytes, three `float4`s.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Quad {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) src: [f32; 4],
}

impl Quad {
    fn bytes(&self, out: &mut Vec<u8>) {
        for value in self.rect.iter().chain(&self.color).chain(&self.src) {
            out.extend_from_slice(&value.to_ne_bytes());
        }
    }
}

/// The device, its queue and the pipelines.
#[derive(Debug)]
pub(crate) struct Gpu {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    solid: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    image: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    /// How long opening it took, in nanoseconds: the device and its queue,
    /// the shader library, and the two pipelines.
    pub(crate) startup_ns: [u64; 3],
}

/// A texture the GPU process samples, and what keeps its memory alive. Fields
/// drop in order, so the texture and any buffer go before the region (G7).
#[derive(Debug)]
pub(crate) struct Resource {
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    _buffer: Option<Retained<ProtocolObject<dyn MTLBuffer>>>,
    _region: Option<Region>,
    _surface: Option<CFRetained<IOSurfaceRef>>,
}

impl Resource {
    pub(crate) fn texture(&self) -> &ProtocolObject<dyn MTLTexture> {
        &self.texture
    }
}

/// A buffer of quads, in shared memory.
#[derive(Debug)]
pub(crate) struct Quads {
    buffer: Retained<ProtocolObject<dyn MTLBuffer>>,
    count: usize,
}

impl Gpu {
    /// Open the default device, and compile the shaders.
    pub(crate) fn new() -> Result<Self, String> {
        let started = time::now();
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no command queue")?;
        let opened = time::now();
        // `S10_SHADER_NONCE` adds a comment, which changes Metal's cache key,
        // so a run can compile cold: the profile must allow the compiler.
        let source = match std::env::var("S10_SHADER_NONCE") {
            Ok(nonce) => format!("// nonce {nonce}\n{SHADERS}"),
            Err(_) => SHADERS.to_owned(),
        };
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(&source), None)
            .map_err(|e| format!("MSL compilation failed: {}", e.localizedDescription()))?;
        let compiled = time::now();
        let solid = pipeline(&device, &library, "fs_solid")?;
        let image = pipeline(&device, &library, "fs_image")?;
        let startup_ns = [
            time::ns(opened - started),
            time::ns(compiled - opened),
            time::ns(time::now() - compiled),
        ];
        Ok(Self {
            device,
            queue,
            solid,
            image,
            startup_ns,
        })
    }

    /// The alignment rows of a linear texture need, for RGBA8.
    pub(crate) fn linear_alignment(&self) -> usize {
        self.device
            .minimumLinearTextureAlignmentForPixelFormat(MTLPixelFormat::RGBA8Unorm)
    }

    pub(crate) fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// A shared buffer holding `quads`.
    pub(crate) fn quads(&self, quads: &[Quad]) -> Result<Quads, String> {
        let mut bytes = Vec::with_capacity(quads.len().max(1) * 48);
        for quad in quads {
            quad.bytes(&mut bytes);
        }
        if bytes.is_empty() {
            bytes.resize(48, 0);
        }
        let start = NonNull::from(bytes.as_slice()).cast();
        // SAFETY: G7, `bytes` is live for the call, which copies `len` bytes
        // into a new shared buffer.
        let buffer = unsafe {
            self.device.newBufferWithBytes_length_options(
                start,
                bytes.len(),
                MTLResourceOptions::StorageModeShared,
            )
        }
        .ok_or("buffer allocation failed")?;
        Ok(Quads {
            buffer,
            count: quads.len(),
        })
    }

    fn linear_descriptor(
        width: usize,
        height: usize,
        storage: MTLStorageMode,
    ) -> Retained<MTLTextureDescriptor> {
        // SAFETY: an RGBA8 2D descriptor of a size the callers checked.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::RGBA8Unorm,
                width,
                height,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(storage);
        descriptor
    }

    fn no_copy_buffer(
        &self,
        region: &Region,
    ) -> Result<Retained<ProtocolObject<dyn MTLBuffer>>, String> {
        // SAFETY: G7, the region is page-aligned and a whole number of pages,
        // and whoever holds the buffer also holds the region, dropped after it.
        unsafe {
            self.device
                .newBufferWithBytesNoCopy_length_options_deallocator(
                    region.start(),
                    region.len(),
                    MTLResourceOptions::StorageModeShared,
                    None,
                )
        }
        .ok_or_else(|| "newBufferWithBytesNoCopy failed".to_owned())
    }

    /// S10-Q2's first path: the mapped region as a buffer, without a copy, read
    /// as a linear texture.
    pub(crate) fn no_copy(
        &self,
        region: Region,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) -> Result<Resource, String> {
        check_layout(&region, width, height, bytes_per_row)?;
        let buffer = self.no_copy_buffer(&region)?;
        let descriptor = Self::linear_descriptor(width, height, MTLStorageMode::Shared);
        let texture = buffer
            .newTextureWithDescriptor_offset_bytesPerRow(&descriptor, 0, bytes_per_row)
            .ok_or("no linear texture over the buffer")?;
        Ok(Resource {
            texture,
            _buffer: Some(buffer),
            _region: Some(region),
            _surface: None,
        })
    }

    /// S10-Q2's second path: one blit from the mapped region into a private
    /// texture; the mapping is released once the blit has finished.
    pub(crate) fn copied(
        &self,
        region: Region,
        width: usize,
        height: usize,
        bytes_per_row: usize,
    ) -> Result<Resource, String> {
        check_layout(&region, width, height, bytes_per_row)?;
        let buffer = self.no_copy_buffer(&region)?;
        let descriptor = Self::linear_descriptor(width, height, MTLStorageMode::Private);
        let texture = self
            .device
            .newTextureWithDescriptor(&descriptor)
            .ok_or("no private texture")?;
        let commands = self.queue.commandBuffer().ok_or("no command buffer")?;
        let blit = commands.blitCommandEncoder().ok_or("no blit encoder")?;
        // SAFETY: the copy reads `height` rows of `bytes_per_row` bytes from the
        // start of the buffer, inside its length (checked by `check_layout`),
        // into the whole texture.
        unsafe {
            blit.copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                &buffer,
                0,
                bytes_per_row,
                bytes_per_row * height,
                MTLSize { width, height, depth: 1 },
                &texture,
                0,
                0,
                MTLOrigin { x: 0, y: 0, z: 0 },
            );
        }
        blit.endEncoding();
        commands.commit();
        commands.waitUntilCompleted();
        drop(buffer);
        drop(region);
        Ok(Resource {
            texture,
            _buffer: None,
            _region: None,
            _surface: None,
        })
    }

    /// The path RV1-09 removes: a texture over the renderer's own IOSurface.
    pub(crate) fn surface_texture(
        &self,
        surface: CFRetained<IOSurfaceRef>,
        width: usize,
        height: usize,
    ) -> Result<Resource, String> {
        // SAFETY: a BGRA8 2D descriptor of the surface's size.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::BGRA8Unorm,
                width,
                height,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let texture = self
            .device
            .newTextureWithDescriptor_iosurface_plane(&descriptor, &surface, 0)
            .ok_or("no texture for the renderer's IOSurface")?;
        Ok(Resource {
            texture,
            _buffer: None,
            _region: None,
            _surface: Some(surface),
        })
    }

    pub(crate) fn record(&self) -> Result<Recording<'_>, String> {
        Ok(Recording {
            gpu: self,
            commands: self.queue.commandBuffer().ok_or("no command buffer")?,
        })
    }
}

fn check_layout(
    region: &Region,
    width: usize,
    height: usize,
    bytes_per_row: usize,
) -> Result<(), String> {
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 || bytes_per_row < width * 4 {
        return Err(format!(
            "a {width}×{height} image with {bytes_per_row}-byte rows is out of range"
        ));
    }
    if bytes_per_row
        .checked_mul(height)
        .is_none_or(|bytes| bytes > region.len())
    {
        return Err("the image doesn't fit its region".to_owned());
    }
    Ok(())
}

/// One image draw: a resource and where to put it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ImageDraw<'a> {
    pub(crate) resource: &'a Resource,
    pub(crate) quad: Quad,
}

/// A frame's GPU work, in one command buffer.
#[derive(Debug)]
pub(crate) struct Recording<'g> {
    gpu: &'g Gpu,
    commands: Retained<ProtocolObject<dyn MTLCommandBuffer>>,
}

impl Recording<'_> {
    /// One tile: clear `target`, draw `solids`, then `images`, with the
    /// tile's top-left corner at `origin` in page pixels.
    pub(crate) fn tile(
        &mut self,
        target: &ProtocolObject<dyn MTLTexture>,
        origin: [f32; 2],
        solids: &Quads,
        images: &[ImageDraw<'_>],
    ) -> Result<(), String> {
        let pass = MTLRenderPassDescriptor::new();
        // SAFETY: attachment 0 of the descriptor's eight.
        let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        color.setTexture(Some(target));
        color.setLoadAction(MTLLoadAction::Clear);
        color.setStoreAction(MTLStoreAction::Store);
        color.setClearColor(MTLClearColor {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
        });
        let encoder = self
            .commands
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or("no render command encoder")?;
        let size = [to_f32(target.width()), to_f32(target.height())];
        encoder.setViewport(MTLViewport {
            originX: 0.0,
            originY: 0.0,
            width: f64::from(size[0]),
            height: f64::from(size[1]),
            znear: 0.0,
            zfar: 1.0,
        });
        let frame = [origin[0], origin[1], size[0], size[1]];
        // SAFETY: 16 bytes at vertex buffer 0, the shaders' `Frame`, which
        // Metal copies during the call.
        unsafe {
            encoder.setVertexBytes_length_atIndex(
                NonNull::from(&frame).cast(),
                size_of_val(&frame),
                0,
            );
        }
        if solids.count > 0 {
            encoder.setRenderPipelineState(&self.gpu.solid);
            // SAFETY: the quads at vertex buffer 1, `count` of them.
            unsafe { encoder.setVertexBuffer_offset_atIndex(Some(&solids.buffer), 0, 1) };
            // SAFETY: four vertices per quad, `count` instances inside the buffer.
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                    solids.count,
                );
            }
        }
        if !images.is_empty() {
            encoder.setRenderPipelineState(&self.gpu.image);
        }
        for draw in images {
            let mut bytes = Vec::with_capacity(48);
            draw.quad.bytes(&mut bytes);
            // SAFETY: one 48-byte quad at vertex buffer 1, copied during the call.
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    NonNull::from(bytes.as_slice()).cast(),
                    bytes.len(),
                    1,
                );
            }
            // SAFETY: fragment texture 0, the one `fs_image` samples.
            unsafe { encoder.setFragmentTexture_atIndex(Some(draw.resource.texture()), 0) };
            // SAFETY: four vertices, one instance, the quad bound above.
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                    1,
                );
            }
        }
        encoder.endEncoding();
        Ok(())
    }

    /// Commit and wait; the GPU time from first work to last, in seconds.
    pub(crate) fn finish(self) -> f64 {
        self.commands.commit();
        self.commands.waitUntilCompleted();
        (self.commands.GPUEndTime() - self.commands.GPUStartTime()).max(0.0)
    }
}

fn pipeline(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    fragment: &str,
) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>, String> {
    let function = |name: &str| {
        library
            .newFunctionWithName(&NSString::from_str(name))
            .ok_or_else(|| format!("the shader library has no function {name}"))
    };
    let descriptor = MTLRenderPipelineDescriptor::new();
    let (vertex_function, fragment_function) = (function("vs_quad")?, function(fragment)?);
    descriptor.setVertexFunction(Some(&*vertex_function));
    descriptor.setFragmentFunction(Some(&*fragment_function));
    // SAFETY: attachment 0 of the descriptor's eight.
    let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    attachment.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    attachment.setBlendingEnabled(true);
    attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
    attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
    attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    device
        .newRenderPipelineStateWithDescriptor_error(&descriptor)
        .map_err(|e| format!("pipeline {fragment}: {}", e.localizedDescription()))
}

#[allow(clippy::cast_precision_loss)] // tile sides are at most 16,384 pixels
fn to_f32(pixels: usize) -> f32 {
    pixels as f32
}
