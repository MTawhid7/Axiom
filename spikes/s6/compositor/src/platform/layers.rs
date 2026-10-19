//! CoreAnimation layers for the compositor thread: tile layers whose contents
//! are IOSurface objects, the containers that scroll and animate them, and explicit
//! transactions (GR2, GR7, GR8, GR11).

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::{CGFloat, CGPoint, CGRect, CGSize};
use objc2_quartz_core::{CALayer, CATransaction, CATransform3D};

use super::surface::Surface;

/// A layer on its way to the compositor thread.
#[derive(Debug)]
pub(crate) struct SendLayer(Retained<CALayer>);

// SAFETY: K7. Once sent, the layer is used only on the receiving thread,
// inside explicit transactions; CoreAnimation guards layer state itself.
unsafe impl Send for SendLayer {}

impl SendLayer {
    pub(crate) fn new(layer: Retained<CALayer>) -> Self {
        Self(layer)
    }

    /// Take the layer, on the compositor thread.
    pub(crate) fn into_layer(self) -> Layer {
        Layer(self.0)
    }
}

/// A layer, used on one thread. Geometry is in points, origin at the top left
/// (the content layer is geometry-flipped, and sublayers inherit it).
#[derive(Debug)]
pub(crate) struct Layer(Retained<CALayer>);

impl Layer {
    /// A new layer with `scale` device pixels per point.
    pub(crate) fn new(scale: f64) -> Self {
        let layer = CALayer::new();
        layer.setContentsScale(scale);
        // Tiles are drawn at their final size; no filtering on composition.
        Self(layer)
    }

    pub(crate) fn add(&self, child: &Layer) {
        self.0.addSublayer(&child.0);
    }

    pub(crate) fn remove(&self) {
        self.0.removeFromSuperlayer();
    }

    /// Place the layer: its frame, in its parent's points.
    pub(crate) fn set_frame(&self, x: f64, y: f64, width: f64, height: f64) {
        self.0
            .setFrame(CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)));
    }

    /// Move the layer's frame origin, keeping its size.
    pub(crate) fn set_origin(&self, x: f64, y: f64) {
        let size = self.0.bounds().size;
        let anchor = self.0.anchorPoint();
        self.0.setPosition(CGPoint::new(
            x + anchor.x * size.width,
            y + anchor.y * size.height,
        ));
    }

    pub(crate) fn set_clips(&self, clips: bool) {
        self.0.setMasksToBounds(clips);
    }

    pub(crate) fn set_opaque(&self, opaque: bool) {
        self.0.setOpaque(opaque);
    }

    pub(crate) fn set_opacity(&self, opacity: f32) {
        self.0.setOpacity(opacity);
    }

    /// Rotate by `radians` about the layer's centre and scale by `scale`.
    pub(crate) fn set_rotation_scale(&self, radians: CGFloat, scale: CGFloat) {
        let rotation = CATransform3D::new_rotation(radians, 0.0, 0.0, 1.0);
        self.0.setTransform(rotation.scale(scale, scale, 1.0));
    }

    /// Show `surface` as the layer's contents.
    pub(crate) fn set_contents(&self, surface: &Surface) {
        let contents: &AnyObject = surface.as_object();
        // SAFETY: K8. The contents object is an IOSurfaceRef.
        unsafe { self.0.setContents(Some(contents)) };
    }

    /// Show nothing.
    pub(crate) fn clear_contents(&self) {
        // SAFETY: K8. No contents at all.
        unsafe { self.0.setContents(None) };
    }
}

/// Run `changes` in one explicit transaction, with implicit animations off,
/// then commit it to the render server.
pub(crate) fn transaction(changes: impl FnOnce()) {
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    changes();
    CATransaction::commit();
    CATransaction::flush();
}
