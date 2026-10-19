//! Metal: the device, the compositor's pipelines (compiled from MSL source at
//! run time, GR4), buffers and textures, and the recording of a frame: one
//! render pass per tile (GR2, GR3), or stencil-then-cover passes for paths
//! (S6-Q4).

use core::ptr::NonNull;

use super::surface;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlendFactor, MTLBuffer, MTLClearColor, MTLColorWriteMask, MTLCommandBuffer,
    MTLCommandEncoder, MTLCommandQueue, MTLCompareFunction, MTLCreateSystemDefaultDevice,
    MTLCullMode, MTLDepthStencilDescriptor, MTLDepthStencilState, MTLDevice, MTLLibrary,
    MTLLoadAction, MTLOrigin, MTLPixelFormat, MTLPrimitiveType, MTLRegion, MTLRenderCommandEncoder,
    MTLRenderPassDescriptor, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
    MTLResourceOptions, MTLSize, MTLStencilDescriptor, MTLStencilOperation, MTLStorageMode,
    MTLStoreAction, MTLTexture, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage, MTLViewport,
};

/// The shader source, compiled by [`Gpu::new`].
const SHADERS: &str = include_str!("../shaders.metal");
/// Samples per pixel for path filling on the GPU.
pub(crate) const PATH_SAMPLES: usize = 4;

/// One tile pass's frame: matches `struct Frame`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Frame {
    origin: [f32; 2],
    size: [f32; 2],
}

/// A rectangle, rounded rectangle, border or rotated square: matches
/// `struct Rect`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub(crate) bounds: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) radii: [f32; 4],
    pub(crate) clip: [f32; 4],
    pub(crate) clip_radii: [f32; 4],
    pub(crate) extra: [f32; 4],
}

/// A box shadow: matches `struct Shadow`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Shadow {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) params: [f32; 4],
}

/// A linear gradient: matches `struct Gradient`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Gradient {
    pub(crate) rect: [f32; 4],
    pub(crate) color0: [f32; 4],
    pub(crate) color1: [f32; 4],
    pub(crate) line: [f32; 4],
    pub(crate) radii: [f32; 4],
}

/// An image: matches `struct Image`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Image {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) radii: [f32; 4],
    pub(crate) extra: [f32; 4],
}

/// A glyph quad: matches `struct Glyph`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Glyph {
    pub(crate) rect: [f32; 4],
    pub(crate) uv: [f32; 4],
    pub(crate) color: [f32; 4],
}

/// A path's cover quad: matches `struct Cover`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Cover {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
}

const _: () = {
    assert!(size_of::<Frame>() == 16);
    assert!(size_of::<Rect>() == 96);
    assert!(size_of::<Shadow>() == 48);
    assert!(size_of::<Gradient>() == 80);
    assert!(size_of::<Image>() == 64);
    assert!(size_of::<Glyph>() == 48);
    assert!(size_of::<Cover>() == 32);
};

/// Plain data the shaders read: `#[repr(C)]` arrays of `f32` or `u32`, with no
/// padding, so any value's bytes are valid to copy.
pub(crate) trait Pod: Copy {}
impl Pod for Rect {}
impl Pod for Shadow {}
impl Pod for Gradient {}
impl Pod for Image {}
impl Pod for Glyph {}
impl Pod for Cover {}
impl Pod for u32 {}
impl Pod for [f32; 2] {}

/// The pipelines, one per kind of item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Rect,
    /// Opaque, unrounded rectangles, without blending.
    RectOpaque,
    Shadow,
    Gradient,
    Image,
    Glyph,
}

/// A shared buffer.
#[derive(Debug)]
pub(crate) struct Buffer {
    raw: Retained<ProtocolObject<dyn MTLBuffer>>,
    bytes: usize,
}

impl Buffer {
    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    /// Copy `items` into the buffer, starting at item `first`.
    pub(crate) fn write<T: Pod>(&self, first: usize, items: &[T]) -> Result<(), String> {
        let offset = first * size_of::<T>();
        let len = size_of_val(items);
        if offset + len > self.bytes {
            return Err(format!(
                "a write of {len} bytes at {offset} overruns a {}-byte buffer",
                self.bytes
            ));
        }
        let destination = self
            .raw
            .contents()
            .as_ptr()
            .cast::<u8>()
            .wrapping_add(offset);
        // SAFETY: K3, K2. No command buffer reading this buffer is in flight;
        // the range lies inside it (checked above); `items` is live for `len`
        // bytes of plain data (`Pod`).
        unsafe { core::ptr::copy_nonoverlapping(items.as_ptr().cast::<u8>(), destination, len) };
        Ok(())
    }
}

/// An index buffer of instance indices, each checked below `bound` when made.
#[derive(Debug)]
pub(crate) struct Indices {
    buffer: Buffer,
    len: usize,
    bound: usize,
}

/// A texture the compositor reads or draws into.
#[derive(Debug)]
pub(crate) struct Texture {
    raw: Retained<ProtocolObject<dyn MTLTexture>>,
}

impl Texture {
    pub(crate) fn raw(&self) -> &ProtocolObject<dyn MTLTexture> {
        &self.raw
    }
}

/// Pixel formats for uploaded textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    /// 8-bit coverage, for the glyph atlas.
    R8,
    /// Premultiplied RGBA, 8 bits per channel, for images.
    Rgba8,
}

/// One instanced draw of `count` items, listed from index `first` of `ids`.
#[derive(Debug)]
pub(crate) struct Draw<'a> {
    pub(crate) kind: Kind,
    pub(crate) instances: &'a Buffer,
    /// How many items `instances` holds.
    pub(crate) instance_count: usize,
    pub(crate) ids: &'a Indices,
    pub(crate) first: usize,
    pub(crate) count: usize,
    pub(crate) texture: Option<&'a Texture>,
}

/// One path: its fan triangles' vertices, then its cover quad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathDraw {
    pub(crate) first_vertex: usize,
    pub(crate) vertex_count: usize,
    pub(crate) cover: usize,
}

#[derive(Debug)]
struct Pipelines {
    rect: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    rect_opaque: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    shadow: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    gradient: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    image: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    glyph: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    path_stencil: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    path_cover: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
}

/// The GPU state.
#[derive(Debug)]
pub(crate) struct Gpu {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pipelines: Pipelines,
    stencil_fill: Retained<ProtocolObject<dyn MTLDepthStencilState>>,
    stencil_cover: Retained<ProtocolObject<dyn MTLDepthStencilState>>,
}

/// How a pipeline blends and samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config {
    blend: bool,
    color_writes: bool,
    samples: usize,
    stencil: bool,
}

const BLENDED: Config = Config {
    blend: true,
    color_writes: true,
    samples: 1,
    stencil: false,
};

impl Gpu {
    /// Open the default device, and compile the shaders and pipelines.
    pub(crate) fn new() -> Result<Self, String> {
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no command queue")?;
        let library = device
            .newLibraryWithSource_options_error(&NSString::from_str(SHADERS), None)
            .map_err(|e| format!("MSL compilation failed: {}", e.localizedDescription()))?;
        let build = |vertex: &str, fragment: &str, config: Config| {
            pipeline(&device, &library, vertex, fragment, config)
        };
        let unblended = Config {
            blend: false,
            ..BLENDED
        };
        let path = Config {
            samples: PATH_SAMPLES,
            stencil: true,
            ..BLENDED
        };
        let pipelines = Pipelines {
            rect: build("vs_rect", "fs_rect", BLENDED)?,
            rect_opaque: build("vs_rect", "fs_rect_opaque", unblended)?,
            shadow: build("vs_shadow", "fs_shadow", BLENDED)?,
            gradient: build("vs_gradient", "fs_gradient", BLENDED)?,
            image: build("vs_image", "fs_image", BLENDED)?,
            glyph: build("vs_glyph", "fs_glyph", BLENDED)?,
            path_stencil: build(
                "vs_path",
                "fs_path_stencil",
                Config {
                    color_writes: false,
                    ..path
                },
            )?,
            path_cover: build("vs_cover", "fs_cover", path)?,
        };
        // Non-zero winding: front faces increment, back faces decrement.
        let fill = stencil_state(
            &device,
            MTLCompareFunction::Always,
            MTLStencilOperation::IncrementWrap,
            MTLStencilOperation::DecrementWrap,
        )?;
        // Cover where the winding isn't zero, and reset the stencil there.
        let cover = stencil_state(
            &device,
            MTLCompareFunction::NotEqual,
            MTLStencilOperation::Zero,
            MTLStencilOperation::Zero,
        )?;
        Ok(Self {
            device,
            queue,
            pipelines,
            stencil_fill: fill,
            stencil_cover: cover,
        })
    }

    pub(crate) fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// A shared buffer holding `items`.
    pub(crate) fn upload<T: Pod>(&self, items: &[T]) -> Result<Buffer, String> {
        let buffer = self.buffer(size_of_val(items).max(16))?;
        buffer.write(0, items)?;
        Ok(buffer)
    }

    /// A zeroed shared buffer of `bytes` bytes.
    pub(crate) fn buffer(&self, bytes: usize) -> Result<Buffer, String> {
        let raw = self
            .device
            .newBufferWithLength_options(bytes.max(16), MTLResourceOptions::StorageModeShared)
            .ok_or("buffer allocation failed")?;
        Ok(Buffer {
            raw,
            bytes: bytes.max(16),
        })
    }

    /// An index buffer of `ids`, each of which must be below `bound`.
    pub(crate) fn indices(&self, ids: &[u32], bound: usize) -> Result<Indices, String> {
        if let Some(bad) = ids.iter().find(|&&id| id as usize >= bound) {
            return Err(format!(
                "instance index {bad} is out of range for {bound} items"
            ));
        }
        Ok(Indices {
            buffer: self.upload(ids)?,
            len: ids.len(),
            bound,
        })
    }

    /// A texture holding `pixels`, `width` × `height`, of `format`.
    pub(crate) fn texture(
        &self,
        format: Format,
        width: usize,
        height: usize,
        pixels: &[u8],
    ) -> Result<Texture, String> {
        let (pixel_format, bytes_per_pixel) = match format {
            Format::R8 => (MTLPixelFormat::R8Unorm, 1),
            Format::Rgba8 => (MTLPixelFormat::RGBA8Unorm, 4),
        };
        check_size(width, height)?;
        if pixels.len() != width * height * bytes_per_pixel {
            return Err("texture data has the wrong length".to_owned());
        }
        // SAFETY: the size is non-zero and within Metal's limits (checked).
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                pixel_format,
                width,
                height,
                false,
            )
        };
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        descriptor.setStorageMode(MTLStorageMode::Shared);
        let raw = self
            .device
            .newTextureWithDescriptor(&descriptor)
            .ok_or("texture allocation failed")?;
        let region = region(width, height);
        // SAFETY: K2. `pixels` holds exactly width × height × bytes per pixel
        // bytes, a row of `width × bytes per pixel` (checked above); Metal
        // copies them during the call.
        unsafe {
            raw.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                NonNull::from(pixels).cast(),
                width * bytes_per_pixel,
            );
        }
        Ok(Texture { raw })
    }

    /// An offscreen BGRA render target the CPU can read back.
    pub(crate) fn target(&self, width: usize, height: usize) -> Result<Texture, String> {
        check_size(width, height)?;
        // SAFETY: the size is non-zero and within Metal's limits (checked).
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
        let raw = self
            .device
            .newTextureWithDescriptor(&descriptor)
            .ok_or("render target allocation failed")?;
        Ok(Texture { raw })
    }

    /// Start recording a frame.
    pub(crate) fn record(&self) -> Result<Recording<'_>, String> {
        Ok(Recording {
            gpu: self,
            commands: self.queue.commandBuffer().ok_or("no command buffer")?,
            passes: 0,
        })
    }
}

/// A frame being recorded into one command buffer.
#[derive(Debug)]
pub(crate) struct Recording<'g> {
    gpu: &'g Gpu,
    commands: Retained<ProtocolObject<dyn MTLCommandBuffer>>,
    passes: usize,
}

/// What a finished frame cost on the GPU.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct GpuTime {
    /// From the command buffer's first GPU work to its last, in seconds.
    pub(crate) seconds: f64,
    pub(crate) passes: usize,
}

impl Recording<'_> {
    /// One tile: clear `target` to `clear` (premultiplied), then run `draws`,
    /// with the tile's top-left corner at `origin` in layer pixels.
    pub(crate) fn tile(
        &mut self,
        target: &ProtocolObject<dyn MTLTexture>,
        origin: [f32; 2],
        clear: [f64; 4],
        draws: &[Draw<'_>],
    ) -> Result<(), String> {
        let pass = MTLRenderPassDescriptor::new();
        // SAFETY: K5. Attachment 0 of 8.
        let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        color.setTexture(Some(target));
        color.setLoadAction(MTLLoadAction::Clear);
        color.setStoreAction(MTLStoreAction::Store);
        color.setClearColor(MTLClearColor {
            red: clear[0],
            green: clear[1],
            blue: clear[2],
            alpha: clear[3],
        });
        let encoder = self
            .commands
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or("no render command encoder")?;
        let frame = Frame {
            origin,
            size: [to_f32(target.width()), to_f32(target.height())],
        };
        set_frame(&encoder, &frame);
        for draw in draws {
            if draw.count == 0 {
                continue;
            }
            self.encode(&encoder, draw)?;
        }
        encoder.endEncoding();
        self.passes += 1;
        Ok(())
    }

    fn encode(
        &self,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
        draw: &Draw<'_>,
    ) -> Result<(), String> {
        let item_size = match draw.kind {
            Kind::Rect | Kind::RectOpaque => size_of::<Rect>(),
            Kind::Shadow => size_of::<Shadow>(),
            Kind::Gradient => size_of::<Gradient>(),
            Kind::Image => size_of::<Image>(),
            Kind::Glyph => size_of::<Glyph>(),
        };
        // K4's checks: every bound index is below the instance count (checked
        // when `ids` was made, against `bound`), the instance buffer holds
        // that many items, and the range lies inside the index buffer.
        if draw.ids.bound > draw.instance_count
            || draw.instance_count * item_size > draw.instances.bytes
            || draw.first + draw.count > draw.ids.len
        {
            return Err("a draw's ranges don't fit its buffers".to_owned());
        }
        let p = &self.gpu.pipelines;
        let (pipeline, needs_texture) = match draw.kind {
            Kind::Rect => (&p.rect, false),
            Kind::RectOpaque => (&p.rect_opaque, false),
            Kind::Shadow => (&p.shadow, false),
            Kind::Gradient => (&p.gradient, false),
            Kind::Image => (&p.image, true),
            Kind::Glyph => (&p.glyph, true),
        };
        encoder.setRenderPipelineState(pipeline);
        // SAFETY: K4, K5. The instance array at vertex buffer 1, also read
        // by the fragment shaders at their buffer 1.
        unsafe { encoder.setVertexBuffer_offset_atIndex(Some(&draw.instances.raw), 0, 1) };
        // SAFETY: K4, K5. As above, for the fragment stage.
        unsafe { encoder.setFragmentBuffer_offset_atIndex(Some(&draw.instances.raw), 0, 1) };
        // SAFETY: K4, K5. The index range starts `first` indices in.
        unsafe {
            encoder.setVertexBuffer_offset_atIndex(
                Some(&draw.ids.buffer.raw),
                draw.first * size_of::<u32>(),
                2,
            );
        }
        if needs_texture {
            let texture = draw
                .texture
                .ok_or("an image or glyph draw needs a texture")?;
            // SAFETY: K4, K5. Fragment texture 0 is the one these shaders sample.
            unsafe { encoder.setFragmentTexture_atIndex(Some(&texture.raw), 0) };
        }
        // SAFETY: K4. Four vertices per instance, generated from vertex_id;
        // the instances index the bound range, checked above.
        unsafe {
            encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                MTLPrimitiveType::TriangleStrip,
                0,
                4,
                draw.count,
            );
        }
        Ok(())
    }

    /// Fill `paths` into `target` on the GPU: for each path, its fan triangles
    /// into a multisampled stencil buffer, then its cover quad where the
    /// winding isn't zero, resolved into `target` (S6-Q4). The multisampled
    /// colour and stencil buffers are memoryless: they live in tile memory.
    pub(crate) fn paths(
        &mut self,
        target: &ProtocolObject<dyn MTLTexture>,
        origin: [f32; 2],
        clear: [f64; 4],
        vertices: &Buffer,
        covers: &Buffer,
        paths: &[PathDraw],
    ) -> Result<(), String> {
        let vertex_capacity = vertices.bytes / size_of::<[f32; 2]>();
        let cover_capacity = covers.bytes / size_of::<Cover>();
        if paths
            .iter()
            .any(|p| p.first_vertex + p.vertex_count > vertex_capacity || p.cover >= cover_capacity)
        {
            return Err("a path's ranges don't fit its buffers".to_owned());
        }
        let (width, height) = (target.width(), target.height());
        let device = &self.gpu.device;
        let msaa = memoryless(device, MTLPixelFormat::BGRA8Unorm, width, height)?;
        let stencil = memoryless(device, MTLPixelFormat::Stencil8, width, height)?;
        let pass = MTLRenderPassDescriptor::new();
        // SAFETY: K5. Attachment 0 of 8.
        let color = unsafe { pass.colorAttachments().objectAtIndexedSubscript(0) };
        color.setTexture(Some(&msaa));
        color.setResolveTexture(Some(target));
        color.setLoadAction(MTLLoadAction::Clear);
        color.setStoreAction(MTLStoreAction::MultisampleResolve);
        color.setClearColor(MTLClearColor {
            red: clear[0],
            green: clear[1],
            blue: clear[2],
            alpha: clear[3],
        });
        let stencil_attachment = pass.stencilAttachment();
        stencil_attachment.setTexture(Some(&stencil));
        stencil_attachment.setLoadAction(MTLLoadAction::Clear);
        stencil_attachment.setStoreAction(MTLStoreAction::DontCare);
        stencil_attachment.setClearStencil(0);
        let encoder = self
            .commands
            .renderCommandEncoderWithDescriptor(&pass)
            .ok_or("no render command encoder")?;
        let frame = Frame {
            origin,
            size: [to_f32(width), to_f32(height)],
        };
        set_frame(&encoder, &frame);
        encoder.setCullMode(MTLCullMode::None);
        encoder.setStencilReferenceValue(0);
        for path in paths {
            encoder.setRenderPipelineState(&self.gpu.pipelines.path_stencil);
            encoder.setDepthStencilState(Some(&self.gpu.stencil_fill));
            // SAFETY: K4, K5. The vertices at vertex buffer 1; the draw reads
            // `vertex_count` of them from `first_vertex`, inside the buffer
            // (checked above).
            unsafe { encoder.setVertexBuffer_offset_atIndex(Some(&vertices.raw), 0, 1) };
            // SAFETY: K4. As above.
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount(
                    MTLPrimitiveType::Triangle,
                    path.first_vertex,
                    path.vertex_count,
                );
            }
            encoder.setRenderPipelineState(&self.gpu.pipelines.path_cover);
            encoder.setDepthStencilState(Some(&self.gpu.stencil_cover));
            // SAFETY: K4, K5. The covers at vertex and fragment buffer 1.
            unsafe { encoder.setVertexBuffer_offset_atIndex(Some(&covers.raw), 0, 1) };
            // SAFETY: K4, K5. As above, for the fragment stage.
            unsafe { encoder.setFragmentBuffer_offset_atIndex(Some(&covers.raw), 0, 1) };
            // SAFETY: K4. One instance, the path's cover, inside the buffer
            // (checked above).
            unsafe {
                encoder.drawPrimitives_vertexStart_vertexCount_instanceCount_baseInstance(
                    MTLPrimitiveType::TriangleStrip,
                    0,
                    4,
                    1,
                    path.cover,
                );
            }
        }
        encoder.endEncoding();
        self.passes += 1;
        Ok(())
    }

    /// Commit the frame and wait for the GPU to finish it.
    pub(crate) fn finish(self) -> GpuTime {
        self.commands.commit();
        self.commands.waitUntilCompleted();
        GpuTime {
            seconds: (self.commands.GPUEndTime() - self.commands.GPUStartTime()).max(0.0),
            passes: self.passes,
        }
    }
}

/// The pixels of `texture`, BGRA, rows top to bottom. Call only after the
/// frames that drew it have finished.
pub(crate) fn read(texture: &ProtocolObject<dyn MTLTexture>) -> Vec<u8> {
    let (width, height) = (texture.width(), texture.height());
    let mut pixels = vec![0_u8; width * height * 4];
    // SAFETY: K3, K2. The texture uses shared storage and every frame that
    // drew it has completed ([`Recording::finish`] waits); `pixels` holds
    // width × height × 4 bytes, `width × 4` per row.
    unsafe {
        texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
            NonNull::from(pixels.as_mut_slice()).cast(),
            width * 4,
            region(width, height),
            0,
        );
    }
    pixels
}

fn set_frame(encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>, frame: &Frame) {
    encoder.setViewport(MTLViewport {
        originX: 0.0,
        originY: 0.0,
        width: f64::from(frame.size[0]),
        height: f64::from(frame.size[1]),
        znear: 0.0,
        zfar: 1.0,
    });
    // SAFETY: K4, K5, K2. One `Frame` at vertex buffer 0, which is all the
    // vertex shaders read there; Metal copies the bytes during the call.
    unsafe {
        encoder.setVertexBytes_length_atIndex(NonNull::from(frame).cast(), size_of::<Frame>(), 0);
    }
}

fn pipeline(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    vertex: &str,
    fragment: &str,
    config: Config,
) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>, String> {
    let function = |name: &str| {
        library
            .newFunctionWithName(&NSString::from_str(name))
            .ok_or_else(|| format!("the shader library has no function {name}"))
    };
    let descriptor = MTLRenderPipelineDescriptor::new();
    let (vertex_function, fragment_function) = (function(vertex)?, function(fragment)?);
    descriptor.setVertexFunction(Some(&*vertex_function));
    descriptor.setFragmentFunction(Some(&*fragment_function));
    descriptor.setRasterSampleCount(config.samples);
    if config.stencil {
        descriptor.setStencilAttachmentPixelFormat(MTLPixelFormat::Stencil8);
    }
    // SAFETY: K5. Attachment 0 of 8.
    let attachment = unsafe { descriptor.colorAttachments().objectAtIndexedSubscript(0) };
    attachment.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    if !config.color_writes {
        attachment.setWriteMask(MTLColorWriteMask::None);
    }
    if config.blend {
        // Premultiplied alpha.
        attachment.setBlendingEnabled(true);
        attachment.setSourceRGBBlendFactor(MTLBlendFactor::One);
        attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
        attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    }
    device
        .newRenderPipelineStateWithDescriptor_error(&descriptor)
        .map_err(|e| format!("pipeline {vertex}/{fragment}: {}", e.localizedDescription()))
}

fn stencil_state(
    device: &ProtocolObject<dyn MTLDevice>,
    compare: MTLCompareFunction,
    front_pass: MTLStencilOperation,
    back_pass: MTLStencilOperation,
) -> Result<Retained<ProtocolObject<dyn MTLDepthStencilState>>, String> {
    let face = |pass: MTLStencilOperation| {
        let descriptor = MTLStencilDescriptor::new();
        descriptor.setStencilCompareFunction(compare);
        descriptor.setStencilFailureOperation(MTLStencilOperation::Keep);
        descriptor.setDepthFailureOperation(MTLStencilOperation::Keep);
        descriptor.setDepthStencilPassOperation(pass);
        descriptor.setReadMask(0xFF);
        descriptor.setWriteMask(0xFF);
        descriptor
    };
    let descriptor = MTLDepthStencilDescriptor::new();
    descriptor.setFrontFaceStencil(Some(&face(front_pass)));
    descriptor.setBackFaceStencil(Some(&face(back_pass)));
    device
        .newDepthStencilStateWithDescriptor(&descriptor)
        .ok_or_else(|| "no depth-stencil state".to_owned())
}

/// A memoryless multisampled attachment: it lives in tile memory only.
fn memoryless(
    device: &ProtocolObject<dyn MTLDevice>,
    format: MTLPixelFormat,
    width: usize,
    height: usize,
) -> Result<Retained<ProtocolObject<dyn MTLTexture>>, String> {
    check_size(width, height)?;
    // SAFETY: the size is non-zero and within Metal's limits (checked).
    let descriptor = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            format, width, height, false,
        )
    };
    descriptor.setTextureType(MTLTextureType::Type2DMultisample);
    // SAFETY: four samples, which every Apple GPU supports for these formats.
    unsafe { descriptor.setSampleCount(PATH_SAMPLES) };
    descriptor.setUsage(MTLTextureUsage::RenderTarget);
    descriptor.setStorageMode(MTLStorageMode::Memoryless);
    device
        .newTextureWithDescriptor(&descriptor)
        .ok_or_else(|| "memoryless attachment allocation failed".to_owned())
}

fn check_size(width: usize, height: usize) -> Result<(), String> {
    if width == 0 || height == 0 || width > surface::MAX_SIDE || height > surface::MAX_SIDE {
        Err(format!("{width}×{height} is outside the texture limits"))
    } else {
        Ok(())
    }
}

fn region(width: usize, height: usize) -> MTLRegion {
    MTLRegion {
        origin: MTLOrigin { x: 0, y: 0, z: 0 },
        size: MTLSize {
            width,
            height,
            depth: 1,
        },
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "pixel dimensions are far below 2^24, so f32 holds them exactly"
)]
fn to_f32(pixels: usize) -> f32 {
    pixels as f32
}

/// Read a texture's single pixel at (`x`, `y`) from BGRA bytes, as RGBA.
pub(crate) fn pixel_rgba(bgra: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let at = (y * width + x) * 4;
    [bgra[at + 2], bgra[at + 1], bgra[at], bgra[at + 3]]
}
