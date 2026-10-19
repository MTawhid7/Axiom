//! Synthetic display lists shaped like pages (S6-Q1): text as glyph quads,
//! rectangles, rounded clips, borders, shadows, gradients and images, with two
//! nested frames, each in its own layer (FT12, GR2). Everything is generated
//! deterministically, so no renderer is needed.
//!
//! Items are drawn by kind, in a fixed order (opaque rectangles, shadows,
//! gradients, rectangles, images, glyphs), and the pages are laid out so that
//! this order is a correct paint order. A real display list interleaves kinds,
//! which needs more, smaller batches (inference; not measured here).

use crate::platform::gpu::{Glyph, Gradient, Image, Rect, Shadow};
use crate::text::Atlas;

/// Text sizes, in device pixels (13, 20 and 28 points at 2×).
pub(crate) const BODY: u32 = 26;
pub(crate) const HEADING: u32 = 40;
pub(crate) const TITLE: u32 = 56;
/// The sizes the atlas holds.
pub(crate) const SIZES: [u32; 3] = [BODY, HEADING, TITLE];
/// Glyph slots reserved for the text field's contents.
pub(crate) const FIELD_SLOTS: usize = 48;

/// The image atlas: its side, and the images packed in it.
pub(crate) const IMAGE_ATLAS: usize = 2048;
const IMAGE_SLOTS: [[usize; 4]; 4] = [
    [0, 0, 1024, 683],
    [1024, 0, 1024, 683],
    [0, 683, 1024, 683],
    [1024, 683, 1024, 683],
];

/// Prose for the pages, written for this spike.
const PROSE: &str = "A compositor draws what the renderers describe, and nothing more. Each \
    frame it takes the display lists that changed, finds the tiles they touch, and redraws only \
    those, so that a blinking caret costs one tile and a still page costs nothing at all. \
    Scrolling moves a layer, which the window server composes without new pixels. Text arrives \
    as glyph quads from an atlas the renderer filled, images as textures it decoded, and the \
    compositor never parses a font or an image itself.";

/// One layer's display list, in its own pixels.
#[derive(Clone, Debug, Default)]
pub(crate) struct List {
    pub(crate) width: f32,
    pub(crate) height: f32,
    /// The clear colour: premultiplied, opaque.
    pub(crate) background: [f32; 4],
    pub(crate) opaque: Vec<Rect>,
    pub(crate) shadows: Vec<Shadow>,
    pub(crate) gradients: Vec<Gradient>,
    pub(crate) rects: Vec<Rect>,
    pub(crate) images: Vec<Image>,
    pub(crate) glyphs: Vec<Glyph>,
}

impl List {
    /// How many items it holds.
    pub(crate) fn items(&self) -> usize {
        self.opaque.len()
            + self.shadows.len()
            + self.gradients.len()
            + self.rects.len()
            + self.images.len()
            + self.glyphs.len()
    }
}

/// A child frame's place in its parent, in the parent's pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Slot {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// The items an update changes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dynamic {
    /// The text field: its box, its first glyph slot, and the caret's index
    /// among the rectangles.
    pub(crate) field: [f32; 4],
    pub(crate) field_glyphs: usize,
    pub(crate) caret: usize,
    /// The spinner: its index among the rectangles, and its box.
    pub(crate) spinner: usize,
    pub(crate) spinner_box: [f32; 4],
}

/// The top-level page, its two nested frames, and their places.
#[derive(Clone, Debug)]
pub(crate) struct Page {
    pub(crate) list: List,
    pub(crate) dynamic: Dynamic,
    pub(crate) frame: List,
    pub(crate) frame_slot: Slot,
    pub(crate) inner: List,
    pub(crate) inner_slot: Slot,
}

/// A small deterministic generator (xorshift64*).
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in [0, 1).
    pub(crate) fn unit(&mut self) -> f32 {
        f32::from(u16::try_from(self.next() >> 48).unwrap_or(0)) / 65_536.0
    }

    /// A value in [lo, hi).
    pub(crate) fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

/// Straight sRGB, 0–255, with alpha, to premultiplied floats.
pub(crate) fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> [f32; 4] {
    let scale = |v: u8| f32::from(v) / 255.0 * alpha;
    [scale(red), scale(green), scale(blue), alpha]
}

fn rect(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Rect {
    Rect {
        bounds: [x, y, w, h],
        color,
        ..Rect::default()
    }
}

fn rounded(x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4]) -> Rect {
    Rect {
        radii: [radius; 4],
        ..rect(x, y, w, h, color)
    }
}

fn border(x: f32, y: f32, w: f32, h: f32, radius: f32, width: f32, color: [f32; 4]) -> Rect {
    Rect {
        extra: [width, 0.0, 0.0, 0.0],
        ..rounded(x, y, w, h, radius, color)
    }
}

fn shadow(x: f32, y: f32, w: f32, h: f32, radius: f32, sigma: f32) -> Shadow {
    Shadow {
        rect: [x, y + sigma * 0.5, w, h],
        color: rgba(0, 0, 0, 0.18),
        params: [radius, sigma, 0.0, 0.0],
    }
}

fn image(x: f32, y: f32, w: f32, h: f32, slot: usize, radius: f32) -> Image {
    let place = IMAGE_SLOTS[slot % IMAGE_SLOTS.len()];
    let side = f32::from(u16::try_from(IMAGE_ATLAS).unwrap_or(u16::MAX));
    let uv = |v: usize| f32::from(u16::try_from(v).unwrap_or(u16::MAX)) / side;
    Image {
        rect: [x, y, w, h],
        uv: [
            uv(place[0]),
            uv(place[1]),
            uv(place[0] + place[2]),
            uv(place[1] + place[3]),
        ],
        radii: [radius; 4],
        extra: [1.0, 0.0, 0.0, 0.0],
    }
}

/// The image atlas's pixels: premultiplied RGBA, four photograph-like images
/// of gradients, soft shapes and grain.
pub(crate) fn image_atlas() -> Vec<u8> {
    let mut pixels = vec![0_u8; IMAGE_ATLAS * IMAGE_ATLAS * 4];
    let mut rng = Rng::new(7);
    for (slot, s) in IMAGE_SLOTS.iter().enumerate() {
        let base = [
            [40.0, 90.0, 160.0],
            [200.0, 120.0, 60.0],
            [60.0, 150.0, 90.0],
            [150.0, 70.0, 140.0],
        ][slot];
        let blobs: Vec<(f32, f32, f32)> = (0..6)
            .map(|_| {
                (
                    rng.range(0.0, 1.0),
                    rng.range(0.0, 1.0),
                    rng.range(0.08, 0.3),
                )
            })
            .collect();
        for y in 0..s[3] {
            for x in 0..s[2] {
                let (u, v) = (to_f32(x) / to_f32(s[2]), to_f32(y) / to_f32(s[3]));
                let glow: f32 = blobs
                    .iter()
                    .map(|&(bx, by, r)| {
                        (1.0 - ((u - bx).powi(2) + (v - by).powi(2)).sqrt() / r).max(0.0)
                    })
                    .sum();
                let grain = rng.range(-6.0, 6.0);
                let at = ((s[1] + y) * IMAGE_ATLAS + s[0] + x) * 4;
                for (c, b) in base.iter().enumerate() {
                    let value = b * (0.55 + 0.45 * v) + 90.0 * glow + grain;
                    pixels[at + c] = to_u8(value);
                }
                pixels[at + 3] = 255;
            }
        }
    }
    pixels
}

/// The page: 2,560 pixels wide and `height` tall, with a header, a field, a
/// spinner, two columns of text, cards, images, and one frame that holds
/// another.
pub(crate) fn page(atlas: &Atlas, height: f32) -> Page {
    let mut list = List {
        width: 2560.0,
        height,
        background: [1.0, 1.0, 1.0, 1.0],
        ..List::default()
    };
    header(atlas, &mut list);
    let dynamic = controls(&mut list);
    let frame_slot = Slot {
        x: 160.0,
        y: 1400.0,
        width: 1200.0,
        height: 800.0,
    };
    let inner_slot = Slot {
        x: 520.0,
        y: 320.0,
        width: 600.0,
        height: 400.0,
    };
    main_column(atlas, &mut list, frame_slot);
    sidebar(atlas, &mut list);
    footer(atlas, &mut list);
    Page {
        list,
        dynamic,
        frame: frame_list(atlas, frame_slot, inner_slot),
        frame_slot,
        inner: inner_list(atlas, inner_slot),
        inner_slot,
    }
}

/// A gradient band, a title, and navigation buttons.
fn header(atlas: &Atlas, list: &mut List) {
    let white = rgba(255, 255, 255, 1.0);
    list.gradients.push(Gradient {
        rect: [0.0, 0.0, 2560.0, 168.0],
        color0: rgba(32, 64, 160, 1.0),
        color1: rgba(96, 48, 150, 1.0),
        line: [0.0, 0.0, 2560.0, 168.0],
        radii: [0.0; 4],
    });
    atlas.line(
        "Axiom compositor",
        TITLE,
        120.0,
        104.0,
        white,
        &mut list.glyphs,
    );
    for (i, label) in ["Overview", "Layers", "Tiles", "Damage", "Paths", "Colour"]
        .iter()
        .enumerate()
    {
        let x = 1180.0 + 200.0 * to_f32(i);
        list.rects.push(rounded(
            x,
            52.0,
            176.0,
            64.0,
            32.0,
            rgba(255, 255, 255, 0.16),
        ));
        list.rects.push(border(
            x,
            52.0,
            176.0,
            64.0,
            32.0,
            2.0,
            rgba(255, 255, 255, 0.5),
        ));
        let width = atlas.width(label, BODY);
        atlas.line(
            label,
            BODY,
            x + (176.0 - width) * 0.5,
            94.0,
            white,
            &mut list.glyphs,
        );
    }
}

/// A text field, with glyph slots reserved for what is typed and a caret, and
/// a spinner: the items updates change.
fn controls(list: &mut List) -> Dynamic {
    let field = [160.0, 220.0, 1100.0, 72.0];
    let [fx, fy, fw, fh] = field;
    list.shadows.push(shadow(fx, fy, fw, fh, 12.0, 6.0));
    list.rects
        .push(rounded(fx, fy, fw, fh, 12.0, rgba(255, 255, 255, 1.0)));
    list.rects
        .push(border(fx, fy, fw, fh, 12.0, 2.0, rgba(60, 90, 200, 1.0)));
    let field_glyphs = list.glyphs.len();
    for _ in 0..FIELD_SLOTS {
        // Placeholders over the field, so the field's tiles bin them;
        // typing (`Stage::type_one`) fills them.
        list.glyphs.push(Glyph {
            rect: [fx + 24.0, fy + 16.0, 1.0, 1.0],
            ..Glyph::default()
        });
    }
    let caret = list.rects.len();
    list.rects
        .push(rect(fx + 24.0, fy + 16.0, 3.0, 40.0, rgba(28, 30, 36, 1.0)));
    // A rounded square that rotates.
    let spinner_box = [1340.0, 216.0, 80.0, 80.0];
    let spinner = list.rects.len();
    list.rects.push(Rect {
        extra: [0.0, 0.3, 0.0, 0.0],
        ..rounded(
            spinner_box[0] + 12.0,
            spinner_box[1] + 12.0,
            56.0,
            56.0,
            10.0,
            rgba(220, 70, 60, 1.0),
        )
    });
    Dynamic {
        field,
        field_glyphs,
        caret,
        spinner,
        spinner_box,
    }
}

/// Sections of headings, paragraphs and cards, around the frame's slot.
fn main_column(atlas: &Atlas, list: &mut List, frame: Slot) {
    let (ink, muted) = (rgba(28, 30, 36, 1.0), rgba(92, 98, 112, 1.0));
    let (x, width) = (160.0, 1500.0);
    let mut y = 360.0;
    let mut section = 0;
    while y < list.height - 600.0 {
        if y < frame.y + frame.height + 40.0 && y + 400.0 > frame.y {
            y = frame.y + frame.height + 80.0;
        }
        let heading = format!("Section {}", section + 1);
        atlas.line(&heading, HEADING, x, y + 40.0, ink, &mut list.glyphs);
        y += 72.0;
        for _ in 0..3 {
            y = atlas.paragraph(PROSE, BODY, x, y, width, muted, &mut list.glyphs) + 24.0;
        }
        if section % 2 == 0 {
            // A card: shadow, rounded body, border, an image clipped to its
            // corners, and a caption.
            let card_height = 520.0;
            list.shadows
                .push(shadow(x, y, width, card_height, 20.0, 14.0));
            list.rects.push(rounded(
                x,
                y,
                width,
                card_height,
                20.0,
                rgba(248, 249, 252, 1.0),
            ));
            list.rects.push(border(
                x,
                y,
                width,
                card_height,
                20.0,
                2.0,
                rgba(210, 214, 224, 1.0),
            ));
            list.images
                .push(image(x + 24.0, y + 24.0, 640.0, 427.0, section / 2, 14.0));
            atlas.paragraph(
                PROSE,
                BODY,
                x + 700.0,
                y + 32.0,
                width - 740.0,
                ink,
                &mut list.glyphs,
            );
            y += card_height + 48.0;
        }
        section += 1;
    }
}

/// Cards with thumbnails, list items and gradient buttons.
fn sidebar(atlas: &Atlas, list: &mut List) {
    let ink = rgba(28, 30, 36, 1.0);
    let mut rng = Rng::new(42);
    let (x, width, card_height) = (1780.0, 640.0, 600.0);
    let mut y = 360.0;
    let mut card = 0;
    while y < list.height - 700.0 {
        list.shadows
            .push(shadow(x, y, width, card_height, 18.0, 10.0));
        list.rects.push(rounded(
            x,
            y,
            width,
            card_height,
            18.0,
            rgba(255, 255, 255, 1.0),
        ));
        list.rects.push(border(
            x,
            y,
            width,
            card_height,
            18.0,
            2.0,
            rgba(222, 226, 234, 1.0),
        ));
        list.images.push(image(
            x + 24.0,
            y + 24.0,
            width - 48.0,
            300.0,
            card + 1,
            12.0,
        ));
        let mut line_y = y + 350.0;
        for i in 0..4 {
            list.rects.push(rounded(
                x + 24.0,
                line_y + 10.0,
                14.0,
                14.0,
                7.0,
                rgba(60, 90, 200, 1.0),
            ));
            let item = format!("Item {} of card {}", i + 1, card + 1);
            atlas.line(&item, BODY, x + 52.0, line_y + 26.0, ink, &mut list.glyphs);
            line_y += 40.0;
        }
        list.gradients.push(Gradient {
            rect: [x + 24.0, y + card_height - 84.0, 220.0, 60.0],
            color0: rgba(40, 170, 120, 1.0),
            color1: rgba(30, 120, 160, 1.0),
            line: [x + 24.0, 0.0, x + 244.0, 0.0],
            radii: [30.0; 4],
        });
        let white = rgba(255, 255, 255, 1.0);
        atlas.line(
            "Open",
            BODY,
            x + 104.0,
            y + card_height - 44.0,
            white,
            &mut list.glyphs,
        );
        y += card_height + rng.range(40.0, 80.0);
        card += 1;
    }
}

/// An opaque footer, drawn without blending.
fn footer(atlas: &Atlas, list: &mut List) {
    let height = list.height;
    list.opaque.push(rect(
        0.0,
        height - 240.0,
        2560.0,
        240.0,
        rgba(24, 26, 32, 1.0),
    ));
    let light = rgba(230, 232, 238, 1.0);
    atlas.line(
        "End of page",
        HEADING,
        160.0,
        height - 110.0,
        light,
        &mut list.glyphs,
    );
}

/// The outer frame: a document of its own, with a slot for the inner frame.
fn frame_list(atlas: &Atlas, slot: Slot, inner: Slot) -> List {
    let mut list = List {
        width: slot.width,
        height: slot.height,
        background: rgba(244, 246, 250, 1.0),
        ..List::default()
    };
    list.gradients.push(Gradient {
        rect: [0.0, 0.0, slot.width, 96.0],
        color0: rgba(230, 120, 60, 1.0),
        color1: rgba(220, 60, 90, 1.0),
        line: [0.0, 0.0, slot.width, 0.0],
        radii: [0.0; 4],
    });
    atlas.line(
        "A frame from another site",
        HEADING,
        40.0,
        62.0,
        rgba(255, 255, 255, 1.0),
        &mut list.glyphs,
    );
    atlas.paragraph(
        PROSE,
        BODY,
        40.0,
        130.0,
        inner.x - 80.0,
        rgba(40, 44, 52, 1.0),
        &mut list.glyphs,
    );
    list.images.push(image(40.0, 560.0, 420.0, 200.0, 2, 10.0));
    list.shadows.push(shadow(
        inner.x,
        inner.y,
        inner.width,
        inner.height,
        8.0,
        8.0,
    ));
    list
}

/// The inner frame.
fn inner_list(atlas: &Atlas, slot: Slot) -> List {
    let mut list = List {
        width: slot.width,
        height: slot.height,
        background: rgba(255, 255, 255, 1.0),
        ..List::default()
    };
    list.rects
        .push(rect(0.0, 0.0, slot.width, 64.0, rgba(40, 44, 52, 1.0)));
    atlas.line(
        "A frame inside that frame",
        BODY,
        24.0,
        42.0,
        rgba(255, 255, 255, 1.0),
        &mut list.glyphs,
    );
    atlas.paragraph(
        PROSE,
        BODY,
        24.0,
        88.0,
        slot.width - 48.0,
        rgba(40, 44, 52, 1.0),
        &mut list.glyphs,
    );
    list
}

pub(crate) fn to_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0–255 first"
)]
fn to_u8(v: f32) -> u8 {
    v.clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_has_every_kind_of_item() {
        let atlas = Atlas::new(&SIZES).expect("Helvetica rasterizes");
        let page = page(&atlas, 8000.0);
        let l = &page.list;
        assert!(!l.opaque.is_empty() && !l.shadows.is_empty() && !l.gradients.is_empty());
        assert!(
            !l.rects.is_empty() && !l.images.is_empty() && l.glyphs.len() > 5000,
            "{}",
            l.glyphs.len()
        );
        assert!(page.frame.items() > 100 && page.inner.items() > 50);
        assert!(page.dynamic.caret < l.rects.len() && page.dynamic.spinner < l.rects.len());
        // Deterministic: the same page twice.
        assert_eq!(page.list.glyphs, super::page(&atlas, 8000.0).list.glyphs);
    }
}
