//! The browser's CoreAnimation layer tree, as S10 built it: one layer per
//! tile, whose contents are IOSurface objects the GPU process rendered. The
//! tree isn't attached to a window.

use objc2::rc::Retained;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_quartz_core::{CALayer, CATransaction};

use super::surface::Received;

/// A layer, used on one thread (L9).
#[derive(Debug)]
pub(crate) struct Layer(Retained<CALayer>);

impl Layer {
    pub(crate) fn new() -> Self {
        Self(CALayer::new())
    }

    pub(crate) fn add(&self, child: &Layer) {
        self.0.addSublayer(&child.0);
    }

    pub(crate) fn set_frame(&self, x: f64, y: f64, side: f64) {
        self.0
            .setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(side, side)));
    }

    /// Show `surface` as the layer's contents, or nothing.
    pub(crate) fn set_contents(&self, surface: Option<&Received>) {
        // SAFETY: L9, the contents object is an IOSurface, or none.
        unsafe { self.0.setContents(surface.map(Received::as_object)) };
    }
}

/// Run `changes` in one explicit transaction, with implicit animations off.
pub(crate) fn transaction(changes: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    changes();
    CATransaction::commit();
}
