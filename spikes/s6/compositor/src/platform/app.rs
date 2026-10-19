//! The application, its one window, and the window's root layer.
//!
//! As in S2, the event loop is pumped by hand (`nextEventMatchingMask:…`), so
//! the caller decides how long the main thread waits, and an idle window
//! costs nothing. Unlike S2, the window's view hosts a plain `CALayer`: the
//! compositor fills it with sublayers whose contents are IOSurface objects (GR2).

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSEventMask,
    NSFloatingWindowLevel, NSScreen, NSView, NSWindow, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::CALayer;

use super::layers::SendLayer;

/// How long [`App::pump`] may wait for the first event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Wait {
    /// Wait up to this many seconds.
    For(f64),
}

/// The application and its window, whose content view hosts a root layer.
#[derive(Debug)]
pub(crate) struct App {
    application: Retained<NSApplication>,
    window: Retained<NSWindow>,
    _view: Retained<NSView>,
    root: Retained<CALayer>,
}

impl App {
    /// Start the application, and show a window of `width` × `height` points
    /// at the top left of the main screen.
    pub(crate) fn new(
        mtm: MainThreadMarker,
        title: &str,
        width: f64,
        height: f64,
    ) -> Result<Self, String> {
        let app = NSApplication::sharedApplication(mtm);
        // A regular app (Dock icon, can be frontmost), even without a bundle.
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();

        let style = NSWindowStyleMask::Titled
            | NSWindowStyleMask::Closable
            | NSWindowStyleMask::Miniaturizable;
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
        // SAFETY: K1. `releasedWhenClosed` is cleared just below, before the
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
        // SAFETY: K1. This is the call that K1 requires.
        unsafe { window.setReleasedWhenClosed(false) };
        window.setTitle(&NSString::from_str(title));

        let view = window
            .contentView()
            .ok_or("the window has no content view")?;
        let root = CALayer::new();
        root.setOpaque(true);
        // A layer-hosting view: give it the layer first, then turn layer
        // backing on. AppKit then leaves the layer's sublayers to us.
        view.setLayer(Some(&root));
        view.setWantsLayer(true);

        // At the top left of the main screen's visible area, below the menu
        // bar.
        let screen = NSScreen::mainScreen(mtm).ok_or("no main screen")?;
        let visible = screen.visibleFrame();
        window.setFrameTopLeftPoint(NSPoint::new(
            visible.origin.x,
            visible.origin.y + visible.size.height,
        ));
        // Measurements need the window on screen. A tool run from a terminal
        // may not be allowed to become the active app (cooperative
        // activation), so the window floats above other apps' windows.
        window.setLevel(NSFloatingWindowLevel);
        // And it may join the current Space even when another app is full
        // screen there.
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        window.makeKeyAndOrderFront(None);
        window.orderFrontRegardless();
        app.activate();
        Ok(Self {
            application: app,
            window,
            _view: view,
            root,
        })
    }

    /// Device pixels per point for the window's screen.
    pub(crate) fn scale(&self) -> f64 {
        self.window.backingScaleFactor()
    }

    /// The window's content size, in points.
    pub(crate) fn content_points(&self) -> (f64, f64) {
        let size = self.window.contentLayoutRect().size;
        (size.width, size.height)
    }

    /// The window's frame and its screen's visible frame, in points.
    pub(crate) fn frames(&self) -> String {
        let f = self.window.frame();
        let s = self.window.screen().map(|s| s.visibleFrame());
        format!(
            "window ({}, {}) {}x{}; screen {:?}",
            f.origin.x,
            f.origin.y,
            f.size.width,
            f.size.height,
            s.map(|s| (s.origin.x, s.origin.y, s.size.width, s.size.height))
        )
    }

    /// Whether the window is ordered in, and whether the window server
    /// reports any of it visible (not occluded).
    pub(crate) fn visibility(&self) -> (bool, bool) {
        (
            self.window.isVisible(),
            self.window
                .occlusionState()
                .contains(objc2_app_kit::NSWindowOcclusionState::Visible),
        )
    }

    /// A new layer filling the root layer, for the compositor thread to fill.
    /// Its geometry is in points, with its origin at the top left.
    pub(crate) fn content_layer(&self) -> SendLayer {
        let layer = CALayer::new();
        layer.setGeometryFlipped(true);
        layer.setFrame(self.root.bounds());
        layer.setContentsScale(self.scale());
        self.root.addSublayer(&layer);
        SendLayer::new(layer)
    }

    /// Handle events: wait for the first one as `wait` allows, then handle
    /// every other pending event without waiting. Returns whether the window
    /// is still open.
    pub(crate) fn pump(&self, wait: Wait) -> bool {
        let first_deadline = match wait {
            Wait::For(seconds) => NSDate::dateWithTimeIntervalSinceNow(seconds),
        };
        let no_wait = NSDate::distantPast();
        // SAFETY: K6.
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
