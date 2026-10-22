//! Metal, cut down from S10's: a device and one pipeline that copies an image
//! into a target of its own size, so the GPU reads every texel, and so every
//! page, of a renderer's memory entry, sampled without a copy as S10-Q2's
//! no-copy path does; and render passes that clear IOSurface tiles, as the
//! GPU process draws them.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBuffer, MTLClearColor, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary, MTLLoadAction, MTLPixelFormat,
    MTLPrimitiveType, MTLRenderCommandEncoder, MTLRenderPassDescriptor,
    MTLRenderPipelineDescriptor, MTLRenderPipelineState, MTLResourceOptions, MTLStorageMode,
    MTLStoreAction, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};

use super::memory::Region;

const SHADERS: &str = r"
#include <metal_stdlib>
using namespace metal;

struct Varyings {
    float4 position [[position]];
    float2 uv;
};

vertex Varyings vs_full(uint vertex_id [[vertex_id]]) {
    float2 corner = float2(float(vertex_id & 1), float(vertex_id >> 1));
    Varyings out;
    out.position = float4(corner * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    out.uv = corner;
    return out;
}

fragment float4 fs_image(Varyings in [[stage_in]], texture2d<float> image [[texture(0)]]) {
    constexpr sampler nearest(coord::normalized, filter::nearest, address::clamp_to_edge);
    return image.sample(nearest, in.uv);
}
";

/// The device, its queue and the pipeline.
#[derive(Debug)]
pub(crate) struct Gpu {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    copy: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

/// A renderer's image, sampled in place: the texture, the buffer under it and
/// the mapping under that. Fields drop in order, so the region goes last
/// (L7).
#[derive(Debug)]
pub(crate) struct Resource {
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    _buffer: Retained<ProtocolObject<dyn MTLBuffer>>,
    region: Region,
}

impl Resource {
    /// The mapping, to read from the CPU.
    pub(crate) fn region(&self) -> &Region {
        &self.region
    }
}

impl Gpu {
    /// Open the default device, and compile the shaders.
    pub(crate) fn new() -> Result<Self, String> {
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no command queue")?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SHADERS), None)
            .map_err(|e| format!("MSL compilation failed: {}", e.localizedDescription()))?;
        let function = |name: &str| {
            library
                .newFunctionWithName(&NSString::from_str(name))
                .ok_or_else(|| format!("the shader library has no function {name}"))
        };
        let descriptor = MTLRenderPipelineDescriptor::new();
        let (vertex, fragment) = (function("vs_full")?, function("fs_image")?);
        descriptor.setVertexFunction(Some(&*vertex));
        descriptor.setFragmentFunction(Some(&*fragment));
        // SAFETY: attachment 0 of the descriptor's eight.
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        let copy = device
            .newRenderPipelineStateWithDescriptor_error(&descriptor)
            .map_err(|e| format!("pipeline: {}", e.localizedDescription()))?;
        Ok(Self {
            device,
            queue,
            copy,
        })
    }

    pub(crate) fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// S10-Q2's no-copy path: the mapped region as a buffer, read as a linear
    /// RGBA8 texture.
    pub(crate) fn no_copy(
        &self,
        region: Region,
        side: usize,
        bytes_per_row: usize,
    ) -> Result<Resource, String> {
        if side == 0 || side > 16_384 || bytes_per_row < side * 4 {
            return Err(format!(
                "a {side}-pixel image with {bytes_per_row}-byte rows"
            ));
        }
        if bytes_per_row
            .checked_mul(side)
            .is_none_or(|bytes| bytes > region.len())
        {
            return Err("the image doesn't fit its region".to_owned());
        }
        // SAFETY: L7, the region is page-aligned and a whole number of pages,
        // and the resource that holds the buffer also holds the region, which
        // it drops after the buffer.
        let buffer = unsafe {
            self.device
                .newBufferWithBytesNoCopy_length_options_deallocator(
                    region.start(),
                    region.len(),
                    MTLResourceOptions::StorageModeShared,
                    None,
                )
        }
        .ok_or("newBufferWithBytesNoCopy failed")?;
        // SAFETY: an RGBA8 2D descriptor of a size checked above.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::RGBA8Unorm,
                side,
                side,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let texture = buffer
            .newTextureWithDescriptor_offset_bytesPerRow(&descriptor, 0, bytes_per_row)
            .ok_or("no linear texture over the buffer")?;
        Ok(Resource {
            texture,
            _buffer: buffer,
            region,
        })
    }

    /// A private BGRA8 render target of `side` × `side`.
    fn target(&self, side: usize) -> Result<Retained<ProtocolObject<dyn MTLTexture>>, String> {
        // SAFETY: a BGRA8 2D descriptor of a size the caller keeps in range.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::BGRA8Unorm,
                side,
                side,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::RenderTarget);
        descriptor.setStorageMode(MTLStorageMode::Private);
        self.device
            .newTextureWithDescriptor(&descriptor)
            .ok_or_else(|| "no render target".to_owned())
    }

    /// Copy each resource into a target of `side` × `side`, one pass each, and
    /// wait for the GPU. The target is freed once the copies are done.
    pub(crate) fn draw_all(&self, resources: &[Resource], side: usize) -> Result<(), String> {
        let target = self.target(side)?;
        let commands = self.queue.commandBuffer().ok_or("no command buffer")?;
        for resource in resources {
            let encoder = encoder(&commands, &target)?;
            encoder.setRenderPipelineState(&self.copy);
            // SAFETY: fragment texture 0, the one `fs_image` samples.
            unsafe { encoder.setFragmentTexture_atIndex(Some(&resource.texture), 0) };
            // SAFETY: four vertices that `vs_full` makes from their IDs, with
            // no buffers bound.
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                );
            }
            encoder.endEncoding();
        }
        commands.commit();
        commands.waitUntilCompleted();
        Ok(())
    }

    /// Clear each texture, one pass each, and wait for the GPU.
    pub(crate) fn clear_all(
        &self,
        targets: &[&ProtocolObject<dyn MTLTexture>],
    ) -> Result<(), String> {
        let commands = self.queue.commandBuffer().ok_or("no command buffer")?;
        for target in targets {
            encoder(&commands, target)?.endEncoding();
        }
        commands.commit();
        commands.waitUntilCompleted();
        Ok(())
    }
}

/// A pass that clears `target` and stores the result.
fn encoder(
    commands: &ProtocolObject<dyn MTLCommandBuffer>,
    target: &ProtocolObject<dyn MTLTexture>,
) -> Result<Retained<ProtocolObject<dyn MTLRenderCommandEncoder>>, String> {
    let pass = MTLRenderPassDescriptor::new();
    // SAFETY: attachment 0 of the descriptor's eight.
    let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
    color.setTexture(Some(target));
    color.setLoadAction(MTLLoadAction::Clear);
    color.setStoreAction(MTLStoreAction::Store);
    color.setClearColor(MTLClearColor {
        red: 0.2,
        green: 0.4,
        blue: 0.6,
        alpha: 1.0,
    });
    commands
        .renderCommandEncoderWithDescriptor(&pass)
        .ok_or_else(|| "no render command encoder".to_owned())
}
