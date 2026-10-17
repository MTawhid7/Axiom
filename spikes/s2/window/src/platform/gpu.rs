//! Metal: the device, a render pipeline compiled from MSL source at run time,
//! the text texture, and frame rendering with optional readback.

use core::ptr::NonNull;
use core::slice;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlendFactor, MTLBlitCommandEncoder, MTLBuffer, MTLClearColor, MTLCommandBuffer,
    MTLCommandEncoder, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice, MTLLibrary,
    MTLLoadAction, MTLOrigin, MTLPixelFormat, MTLPrimitiveType, MTLRegion, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
    MTLResourceOptions, MTLSize, MTLStorageMode, MTLStoreAction, MTLTexture, MTLTextureDescriptor,
    MTLTextureUsage,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};

use super::text::Coverage;

/// The shader source, compiled when [`Gpu::new`] runs.
const SHADER: &str = include_str!("../shader.metal");

/// The background, as a clear colour.
pub(crate) const CLEAR: MTLClearColor = MTLClearColor {
    red: 0.08,
    green: 0.09,
    blue: 0.11,
    alpha: 1.0,
};

/// The text colour (straight RGBA).
pub(crate) const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// One quad's placement and colour. It must match `struct Quad` in
/// `shader.metal`: `float2` is 8-byte aligned and `float4` 16-byte aligned,
/// hence the padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Quad {
    origin: [f32; 2],
    size: [f32; 2],
    viewport: [f32; 2],
    padding: [f32; 2],
    color: [f32; 4],
}

const _: () = assert!(size_of::<Quad>() == 48, "must match the MSL struct's size");

/// A frame's pixels, copied back to the CPU: BGRA, 8 bits per channel, rows
/// top to bottom, no row padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Readback {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bgra: Vec<u8>,
}

#[derive(Debug)]
struct TextTexture {
    texture: Retained<ProtocolObject<dyn MTLTexture>>,
    width: usize,
    height: usize,
}

/// The GPU state for drawing frames.
#[derive(Debug)]
pub(crate) struct Gpu {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    text: Option<TextTexture>,
}

impl Gpu {
    /// Open the default device, and compile the shaders and pipeline.
    pub(crate) fn new() -> Result<Self, String> {
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no command queue")?;

        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SHADER), None)
            .map_err(|e| format!("MSL compilation failed: {}", e.localizedDescription()))?;
        let function = |name: &str| {
            library
                .newFunctionWithName(&NSString::from_str(name))
                .ok_or_else(|| format!("the shader library has no function {name}"))
        };
        let descriptor = MTLRenderPipelineDescriptor::new();
        let (vertex, fragment) = (function("vs_main")?, function("fs_main")?);
        descriptor.setVertexFunction(Some(&vertex));
        descriptor.setFragmentFunction(Some(&fragment));
        // SAFETY: P5. Attachment 0 of 8.
        let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
        attachment.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        // Premultiplied alpha: the fragment shader returns rgb × alpha.
        attachment.setBlendingEnabled(true);
        attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
        attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
        attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        let pipeline = device
            .newRenderPipelineStateWithDescriptor_error(&descriptor)
            .map_err(|e| format!("pipeline creation failed: {}", e.localizedDescription()))?;

        Ok(Self {
            device,
            queue,
            pipeline,
            text: None,
        })
    }

    pub(crate) fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// Upload a coverage mask as the text texture.
    pub(crate) fn set_text(&mut self, coverage: &Coverage) -> Result<(), String> {
        if coverage.width == 0 || coverage.height == 0 || coverage.width > 16_384 {
            return Err(format!(
                "text mask of {}×{} pixels is outside Metal's texture limits",
                coverage.width, coverage.height
            ));
        }
        // SAFETY: the width and height are non-zero and within the 16,384-pixel
        // limit for 2D textures on Apple GPUs (checked above; the height is
        // one line of text).
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::R8Unorm,
                coverage.width,
                coverage.height,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let texture = self
            .device
            .newTextureWithDescriptor(&descriptor)
            .ok_or("text texture allocation failed")?;
        let region = MTLRegion {
            origin: MTLOrigin { x: 0, y: 0, z: 0 },
            size: MTLSize {
                width: coverage.width,
                height: coverage.height,
                depth: 1,
            },
        };
        assert_eq!(coverage.pixels.len(), coverage.width * coverage.height);
        // SAFETY: P2. `coverage.pixels` holds exactly width × height bytes,
        // `width` per row (asserted above); Metal copies them during the call.
        unsafe {
            texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                NonNull::from(coverage.pixels.as_slice()).cast(),
                coverage.width,
            );
        }
        self.text = Some(TextTexture {
            texture,
            width: coverage.width,
            height: coverage.height,
        });
        Ok(())
    }

    /// Draw one frame into `layer`'s next drawable, with the text's top-left
    /// corner at `text_origin` (drawable pixels), and present it. With
    /// `readback`, also return the frame's pixels. Waits for the GPU.
    pub(crate) fn render(
        &self,
        layer: &CAMetalLayer,
        text_origin: [f32; 2],
        readback: bool,
    ) -> Result<Option<Readback>, String> {
        let drawable = layer.nextDrawable().ok_or("no drawable available")?;
        let target = drawable.texture();
        let (width, height) = (target.width(), target.height());
        let commands = self.queue.commandBuffer().ok_or("no command buffer")?;

        let pass = MTLRenderPassDescriptor::new();
        // SAFETY: P5. Attachment 0 of 8.
        let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        color.setTexture(Some(&target));
        color.setLoadAction(MTLLoadAction::Clear);
        color.setStoreAction(MTLStoreAction::Store);
        color.setClearColor(CLEAR);
        let encoder = commands
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or("no render command encoder")?;
        if let Some(text) = &self.text {
            encoder.setRenderPipelineState(&self.pipeline);
            let quad = Quad {
                origin: text_origin,
                size: [to_f32(text.width), to_f32(text.height)],
                viewport: [to_f32(width), to_f32(height)],
                padding: [0.0; 2],
                color: TEXT_COLOR,
            };
            // SAFETY: P4, P5. Exactly one `Quad` at vertex buffer 0, which is
            // all the vertex shader reads; Metal copies the bytes during the
            // call, so a stack value is enough.
            unsafe {
                encoder.setVertexBytes_length_atIndex(
                    NonNull::from(&quad).cast(),
                    size_of::<Quad>(),
                    0,
                );
            }
            // SAFETY: P4, P5. Fragment texture 0 is the one texture the
            // fragment shader samples.
            unsafe { encoder.setFragmentTexture_atIndex(Some(&text.texture), 0) };
            // SAFETY: P4. Four vertices, generated from vertex_id; no vertex
            // buffer is read.
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                );
            }
        }
        encoder.endEncoding();

        let copy = if readback {
            Some(self.encode_readback(&commands, &target, width, height)?)
        } else {
            None
        };
        commands.presentDrawable(ProtocolObject::from_ref(&*drawable));
        commands.commit();
        commands.waitUntilCompleted();

        Ok(copy.map(|buffer| {
            let length = width * 4 * height;
            // SAFETY: P3. The command buffer that wrote `buffer` has completed,
            // the buffer uses shared storage, and it holds `length` bytes
            // (allocated in `encode_readback`).
            let bytes =
                unsafe { slice::from_raw_parts(buffer.contents().as_ptr().cast::<u8>(), length) };
            Readback {
                width,
                height,
                bgra: bytes.to_vec(),
            }
        }))
    }

    /// Encode a copy of all of `target` into a new shared buffer.
    fn encode_readback(
        &self,
        commands: &ProtocolObject<dyn MTLCommandBuffer>,
        target: &ProtocolObject<dyn MTLTexture>,
        width: usize,
        height: usize,
    ) -> Result<Retained<ProtocolObject<dyn MTLBuffer>>, String> {
        let bytes_per_row = width * 4;
        let length = bytes_per_row * height;
        let buffer = self
            .device
            .newBufferWithLength_options(length, MTLResourceOptions::StorageModeShared)
            .ok_or("readback buffer allocation failed")?;
        let blit = commands
            .blitCommandEncoder()
            .ok_or("no blit command encoder")?;
        // SAFETY: P4. The copy covers exactly the `width` × `height` texture
        // (slice 0, level 0, origin 0) and writes `bytes_per_row` × `height`
        // = `length` bytes, the buffer's whole length.
        unsafe {
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
                target,
                0,
                0,
                MTLOrigin { x: 0, y: 0, z: 0 },
                MTLSize {
                    width,
                    height,
                    depth: 1,
                },
                &buffer,
                0,
                bytes_per_row,
                length,
            );
        }
        blit.endEncoding();
        Ok(buffer)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "pixel dimensions are far below 2^24, so f32 holds them exactly"
)]
fn to_f32(pixels: usize) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::text::rasterize_line;

    /// Device creation, run-time MSL compilation, pipeline creation and
    /// texture upload need no window, so they run under `cargo test`.
    #[test]
    fn shaders_compile_and_text_uploads() {
        let mut gpu = Gpu::new().expect("a Metal device, and shaders that compile");
        let mask = rasterize_line("Arbor", "Helvetica", 32.0).expect("CoreText is available");
        gpu.set_text(&mask).expect("the mask uploads");
        let text = gpu.text.as_ref().expect("a text texture");
        assert_eq!((text.width, text.height), (mask.width, mask.height));
        assert_eq!(
            (text.texture.width(), text.texture.height()),
            (mask.width, mask.height)
        );
        let empty = Coverage {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        };
        assert!(
            gpu.set_text(&empty).is_err(),
            "Metal rejects empty textures"
        );
    }
}
