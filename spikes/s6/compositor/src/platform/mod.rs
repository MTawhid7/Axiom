//! The platform layer: every call into AppKit, CoreAnimation, IOSurface,
//! Metal and the C library. It is the only place in this crate where `unsafe`
//! is allowed (`tools/unsafe-policy.txt`). S2's window spike
//! (`spikes/s2/window/src/platform/`) is the model. Every `unsafe` block cites
//! one of these invariants:
//!
//! - **K1, window ownership.** A window is created with
//!   `releasedWhenClosed = false` before anything else can close it, so
//!   closing it never frees an object that Rust still holds a `Retained` to
//!   (objc2's documented requirement for `-[NSWindow initWithContentRect:…]`).
//! - **K2, CPU memory lent to a framework.** Every raw pointer passed to Metal
//!   or the C library points into a live Rust allocation of at least the
//!   declared length, which outlives the call.
//! - **K3, shared GPU memory.** A shared `MTLBuffer` is written on the CPU only
//!   while no command buffer that reads it is in flight (the compositor waits
//!   for each frame's command buffer), read only after the command buffer that
//!   wrote it has completed, and in both cases only within its length.
//! - **K4, shader resource bounds.** Each draw binds exactly what
//!   `shaders.metal` reads for its pipeline: a `Frame` at vertex buffer 0, the
//!   instance array at vertex buffer 1, the instance indices at vertex buffer 2,
//!   and for images and glyphs one texture at fragment texture 0. Every index
//!   in the bound index range is below the instance array's length, and the
//!   range lies inside the index buffer (both checked by `Gpu::draw` before
//!   encoding). Path draws read vertex buffer 1 within the vertex count drawn.
//! - **K5, indices.** Indexed subscripts stay below Metal's documented limits:
//!   colour attachment 0 of 8; buffer slots 0–2 and texture slot 0 of 31.
//! - **K6, framework constants.** Extern statics such as
//!   `NSDefaultRunLoopMode`, `kIOSurfaceWidth` and `kCARendererColorSpace` are
//!   initialized when their framework loads (they are linked here), and are
//!   never written.
//! - **K7, layers across threads.** A layer handed to the compositor thread
//!   ([`layers::SendLayer`]) is afterwards used only on that thread, and only
//!   inside explicit transactions that the thread begins and commits.
//!   CoreAnimation guards layer state with its own lock, and transactions
//!   belong to the thread that begins them (Apple's `CATransaction`
//!   documentation); Firefox's compositing thread commits its layers this way.
//! - **K8, documented object types.** A layer's `contents` is set only to an
//!   `IOSurfaceRef`, a type CoreAnimation accepts as layer contents. IOSurface
//!   property dictionaries map IOSurface's keys to `CFNumber`s, and the colour
//!   space key to a colour space's property list, as `IOSurfaceRef.h`
//!   documents. `CARenderer`'s options map `kCARendererColorSpace` to a
//!   `CGColorSpace`.
//! - **K9, C declarations.** The foreign functions declared here
//!   (`clock_gettime_nsec_np`, `proc_pid_rusage`) match the SDK's headers
//!   (`<_time.h>`, `<libproc.h>`, `<sys/resource.h>`), and their constants are
//!   the headers' values.
#![allow(unsafe_code)]

pub(crate) mod app;
pub(crate) mod clock;
pub(crate) mod gpu;
pub(crate) mod layers;
pub(crate) mod surface;
