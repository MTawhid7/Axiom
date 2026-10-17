//! The application, its one window, and the window's Metal layer.
//!
//! The event loop is pumped by hand (`nextEventMatchingMask:…`) rather than
//! with `-[NSApplication run]`. The caller then decides when to draw. When
//! there is nothing to draw, the loop blocks inside AppKit until an event
//! arrives, so an idle window costs no CPU.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEventMask, NSView,
    NSWindow, NSWindowStyleMask,
};
use objc2_core_foundation::CGSize;
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, NSString};
use objc2_metal::{MTLDevice, MTLPixelFormat};
use objc2_quartz_core::{CALayer, CAMetalLayer};

/// How long [`App::pump`] may wait for the first event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Wait {
    /// Handle pending events, if any, and return at once.
    No,
    /// Wait up to this many seconds.
    For(f64),
    /// Wait until an event arrives.
    Forever,
}

/// The application and its window. The window's content view hosts a
/// `CAMetalLayer`.
#[derive(Debug)]
pub(crate) struct App {
    application: Retained<NSApplication>,
    window: Retained<NSWindow>,
    view: Retained<NSView>,
    layer: Retained<CAMetalLayer>,
}

impl App {
    /// Start the application, and show a centred window of `width` × `height`
    /// points whose layer draws with `device`.
    pub(crate) fn new(
        mtm: MainThreadMarker,
        title: &str,
        width: f64,
        height: f64,
        device: &ProtocolObject<dyn MTLDevice>,
    ) -> Result<Self, String> {
        let app = NSApplication::sharedApplication(mtm);
        // A regular app (Dock icon, can be frontmost), even without a bundle.
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();

        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable
            | NSWindowStyleMask::Resizable;
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
        // SAFETY: P1. `releasedWhenClosed` is cleared just below, before the
        // window is shown and so before anything can close it.
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // SAFETY: P1. This is the call that P1 requires.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(title));

        let view = window
            .contentView()
            .ok_or("the window has no content view")?;
        let layer = CAMetalLayer::new();
        layer.setDevice(Some(device));
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        // Blits may read drawables, so the smoke test can check what was drawn.
        layer.setFramebufferOnly(false);
        // A layer-hosting view: give it the layer first, then turn layer
        // backing on. AppKit leaves a hosted layer's contents to us.
        let hosted: &CALayer = &layer;
        view.setLayer(Some(hosted));
        view.setWantsLayer(true);

        window.center();
        window.makeKeyAndOrderFront(None);
        app.activate();
        Ok(Self {
            application: app,
            window,
            view,
            layer,
        })
    }

    pub(crate) fn layer(&self) -> &CAMetalLayer {
        &self.layer
    }

    /// Device pixels per point for the window's current screen.
    pub(crate) fn scale(&self) -> f64 {
        self.window.backingScaleFactor()
    }

    /// Size the layer's drawables to the view, in device pixels, and return
    /// that size.
    pub(crate) fn sync_drawable_size(&self) -> (f64, f64) {
        let scale = self.scale();
        let points = self.view.bounds().size;
        let pixels = CGSize::new(
            (points.width * scale).round(),
            (points.height * scale).round(),
        );
        self.layer.setContentsScale(scale);
        self.layer.setDrawableSize(pixels);
        (pixels.width, pixels.height)
    }

    /// Handle events: wait for the first one as `wait` allows, then handle
    /// every other pending event without waiting. Returns whether the window
    /// is still open.
    pub(crate) fn pump(&self, wait: Wait) -> bool {
        let first_deadline = match wait {
            Wait::No => NSDate::distantPast(),
            Wait::For(seconds) => NSDate::dateWithTimeIntervalSinceNow(seconds),
            Wait::Forever => NSDate::distantFuture(),
        };
        let no_wait = NSDate::distantPast();
        // SAFETY: P6.
        let mode = unsafe { NSDefaultRunLoopMode };
        let mut deadline = &first_deadline;
        while let Some(event) = self
            .application
            .nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(deadline),
                mode,
                true,
            )
        {
            self.application.sendEvent(&event);
            deadline = &no_wait;
        }
        self.application.updateWindows();
        self.window.isVisible()
    }
}
