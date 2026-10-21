//! The browser's CoreAnimation layer tree: one layer per tile, whose contents
//! are IOSurface objects the GPU process rendered. The tree isn't attached to a
//! window, so a commit costs what the browser's side costs, without the
//! window server's composition, which both topologies share (S6 measured it).

use objc2::rc::Retained;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_quartz_core::{CALayer, CATransaction};

use super::surface::Received;

/// A layer, used on one thread (G9).
#[derive(Debug)]
pub(crate) struct Layer(Retained<CALayer>);

impl Layer {
    pub(crate) fn new() -> Self {
        Self(CALayer::new())
    }

    pub(crate) fn add(&self, child: &Layer) {
        self.0.addSublayer(&child.0);
    }

    pub(crate) fn set_frame(&self, x: f64, y: f64, width: f64, height: f64) {
        self.0
            .setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
    }

    /// Show `surface` as the layer's contents.
    pub(crate) fn set_contents(&self, surface: &Received) {
        // SAFETY: G9, the contents object is an IOSurface.
        unsafe { self.0.setContents(Some(surface.as_object())) };
    }
}

/// Run `changes` in one explicit transaction, with implicit animations off.
pub(crate) fn transaction(changes: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    changes();
    CATransaction::commit();
}
