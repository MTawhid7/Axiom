//! S6-Q4: path-heavy content (icons, a chart, a map tile), filled two ways:
//! - on the CPU, as a renderer would, by `vello_cpu` (and, for comparison,
//!   `zeno`), into an image resource that is then uploaded (GR5 as written);
//! - on the GPU, in the compositor: flattened on the CPU into triangle fans,
//!   then stencil-then-cover with 4× multisampling (the amendment GR5 would
//!   propose). The path code is this spike's own, as GR5 requires of path code
//!   in the browser.
//!
//! All paths fill with the non-zero rule.

use vello_cpu::color::{AlphaColor, Srgb};
use vello_cpu::kurbo::{BezPath, PathEl, Point, Rect as KRect, Shape};

use crate::platform::gpu::{Cover, PathDraw};
use crate::scene::Rng;

/// Flattening tolerance for the GPU path, in pixels.
pub(crate) const TOLERANCE: f64 = 0.2;

/// One filled path and its colour (straight RGBA, 0–255).
#[derive(Clone, Debug)]
pub(crate) struct Item {
    pub(crate) path: BezPath,
    pub(crate) color: [u8; 4],
}

/// A canvas of paths.
#[derive(Clone, Debug)]
pub(crate) struct Content {
    pub(crate) name: &'static str,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) items: Vec<Item>,
}

impl Content {
    /// Segments in all paths (lines and curves), a measure of complexity.
    pub(crate) fn segments(&self) -> usize {
        self.items
            .iter()
            .map(|i| {
                i.path
                    .elements()
                    .iter()
                    .filter(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath))
                    .count()
            })
            .sum()
    }
}

/// The three test contents.
pub(crate) fn contents() -> Vec<Content> {
    vec![icons(), chart(), map_tile()]
}

/// 64 icons of 96 pixels: circles, stars, gears with holes, rounded squares
/// and arrows, in an 8 × 8 grid.
fn icons() -> Content {
    let mut items = Vec::new();
    let palette = [
        [52, 120, 220, 255],
        [230, 90, 70, 255],
        [40, 170, 120, 255],
        [150, 80, 200, 255],
        [240, 170, 40, 255],
    ];
    for i in 0..64_usize {
        let (cx, cy) = (to_f64(i % 8) * 128.0 + 64.0, to_f64(i / 8) * 128.0 + 64.0);
        let color = palette[i % 5];
        let path = match i % 5 {
            0 => circle(cx, cy, 44.0),
            1 => star(cx, cy, 46.0, 20.0, 5 + i % 3),
            2 => {
                let mut gear = gear(cx, cy, 46.0, 36.0, 10);
                // A hole: the inner circle wound the other way.
                gear.extend(circle(cx, cy, 14.0).reverse_subpaths());
                gear
            }
            3 => KRect::new(cx - 42.0, cy - 42.0, cx + 42.0, cy + 42.0)
                .to_rounded_rect(14.0)
                .to_path(0.1),
            _ => arrow(cx, cy, 44.0),
        };
        items.push(Item { path, color });
        // A highlight on every icon: a smaller translucent shape.
        items.push(Item {
            path: circle(cx - 14.0, cy - 14.0, 12.0),
            color: [255, 255, 255, 110],
        });
    }
    Content {
        name: "icons",
        width: 1024,
        height: 1024,
        items,
    }
}

/// A chart, 2,048 × 768: grid lines, a filled area under a 1,200-point
/// series, and three series drawn as 3-pixel strips.
fn chart() -> Content {
    let width = 2048.0;
    let mut items = Vec::new();
    for g in 0..9 {
        let y = 40.0 + f64::from(g) * 84.0;
        items.push(Item {
            path: KRect::new(40.0, y, width - 40.0, y + 1.5).to_path(0.1),
            color: [200, 204, 212, 255],
        });
    }
    let mut rng = Rng::new(3);
    let series = |rng: &mut Rng, base: f64, amplitude: f64| -> Vec<Point> {
        let mut value = 0.0_f64;
        (0..1200)
            .map(|i| {
                value = (value + f64::from(rng.range(-1.0, 1.0)) * 0.08).clamp(-1.0, 1.0);
                let x = 40.0 + f64::from(i) * (width - 80.0) / 1199.0;
                let wave = (f64::from(i) * 0.02).sin() * 0.4;
                Point::new(x, base - amplitude * (value + wave))
            })
            .collect()
    };
    let area = series(&mut rng, 520.0, 160.0);
    let mut fill = BezPath::new();
    fill.move_to(Point::new(area[0].x, 700.0));
    for p in &area {
        fill.line_to(*p);
    }
    fill.line_to(Point::new(area[area.len() - 1].x, 700.0));
    fill.close_path();
    items.push(Item {
        path: fill,
        color: [52, 120, 220, 90],
    });
    for (k, color) in [[52, 120, 220, 255], [230, 90, 70, 255], [40, 170, 120, 255]]
        .into_iter()
        .enumerate()
    {
        let points = if k == 0 {
            area.clone()
        } else {
            series(
                &mut rng,
                300.0 + 120.0 * f64::from(u8::try_from(k).unwrap_or(0)),
                120.0,
            )
        };
        items.push(Item {
            path: strip(&points, 1.5),
            color,
        });
    }
    Content {
        name: "chart",
        width: 2048,
        height: 768,
        items,
    }
}

/// A map tile, 1,024 × 1,024: water with a curved shore, about 500 building
/// blocks, and roads as strips.
fn map_tile() -> Content {
    let mut items = Vec::new();
    let mut rng = Rng::new(11);
    let mut water = BezPath::new();
    water.move_to(Point::new(0.0, 640.0));
    water.curve_to(
        Point::new(260.0, 560.0),
        Point::new(420.0, 820.0),
        Point::new(700.0, 700.0),
    );
    water.curve_to(
        Point::new(860.0, 630.0),
        Point::new(960.0, 760.0),
        Point::new(1024.0, 720.0),
    );
    water.line_to(Point::new(1024.0, 1024.0));
    water.line_to(Point::new(0.0, 1024.0));
    water.close_path();
    items.push(Item {
        path: water,
        color: [150, 190, 230, 255],
    });
    for by in 0..22 {
        for bx in 0..24 {
            let (x, y) = (f64::from(bx) * 42.0 + 8.0, f64::from(by) * 28.0 + 8.0);
            if rng.unit() < 0.06 {
                continue;
            }
            let j = |rng: &mut Rng| f64::from(rng.range(-3.0, 3.0));
            let mut block = BezPath::new();
            block.move_to(Point::new(x + j(&mut rng), y + j(&mut rng)));
            block.line_to(Point::new(x + 32.0 + j(&mut rng), y + j(&mut rng)));
            block.line_to(Point::new(x + 32.0 + j(&mut rng), y + 20.0 + j(&mut rng)));
            block.line_to(Point::new(x + j(&mut rng), y + 20.0 + j(&mut rng)));
            block.close_path();
            let shade = 205 + u8::try_from(rng.next() % 30).unwrap_or(0);
            items.push(Item {
                path: block,
                color: [shade, shade - 6, shade - 14, 255],
            });
        }
    }
    for r in 0..14 {
        let mut points = Vec::new();
        let horizontal = r % 2 == 0;
        let base = f64::from(r / 2) * 150.0 + 60.0;
        for i in 0..=40 {
            let t = f64::from(i) * 1024.0 / 40.0;
            let wobble = (t * 0.01 + f64::from(r)).sin() * 18.0;
            points.push(if horizontal {
                Point::new(t, base + wobble)
            } else {
                Point::new(base + wobble, t)
            });
        }
        items.push(Item {
            path: strip(&points, if r % 3 == 0 { 8.0 } else { 4.0 }),
            color: [255, 255, 255, 255],
        });
    }
    Content {
        name: "map tile",
        width: 1024,
        height: 1024,
        items,
    }
}

fn circle(cx: f64, cy: f64, r: f64) -> BezPath {
    vello_cpu::kurbo::Circle::new(Point::new(cx, cy), r).to_path(0.1)
}

fn star(cx: f64, cy: f64, outer: f64, inner: f64, points: usize) -> BezPath {
    let mut path = BezPath::new();
    let n = points * 2;
    for k in 0..n {
        let r = if k % 2 == 0 { outer } else { inner };
        let a = std::f64::consts::TAU * to_f64(k) / to_f64(n) - std::f64::consts::FRAC_PI_2;
        let p = Point::new(cx + r * a.cos(), cy + r * a.sin());
        if k == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

fn gear(cx: f64, cy: f64, outer: f64, inner: f64, teeth: usize) -> BezPath {
    let mut path = BezPath::new();
    let n = teeth * 4;
    for k in 0..n {
        let r = if (k / 2) % 2 == 0 { outer } else { inner };
        let a = std::f64::consts::TAU * to_f64(k) / to_f64(n);
        let p = Point::new(cx + r * a.cos(), cy + r * a.sin());
        if k == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

fn arrow(cx: f64, cy: f64, s: f64) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(Point::new(cx - s, cy - s * 0.25));
    path.line_to(Point::new(cx, cy - s * 0.25));
    path.line_to(Point::new(cx, cy - s * 0.7));
    path.quad_to(
        Point::new(cx + s * 0.5, cy - s * 0.2),
        Point::new(cx + s, cy),
    );
    path.quad_to(
        Point::new(cx + s * 0.5, cy + s * 0.2),
        Point::new(cx, cy + s * 0.7),
    );
    path.line_to(Point::new(cx, cy + s * 0.25));
    path.line_to(Point::new(cx - s, cy + s * 0.25));
    path.close_path();
    path
}

/// A polyline as a filled strip of half-width `half`: each segment a quad,
/// all in one path (overlaps fill once under the non-zero rule only where they
/// wind the same way, as consecutive segment quads do).
fn strip(points: &[Point], half: f64) -> BezPath {
    let mut path = BezPath::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let d = b - a;
        let len = d.hypot().max(1e-9);
        let n = vello_cpu::kurbo::Vec2::new(-d.y / len * half, d.x / len * half);
        path.move_to(a + n);
        path.line_to(b + n);
        path.line_to(b - n);
        path.line_to(a - n);
        path.close_path();
    }
    path
}

/// Premultiplied RGBA from straight RGBA.
pub(crate) fn premultiply(c: [u8; 4]) -> [u8; 4] {
    let a = u16::from(c[3]);
    let m = |v: u8| u8::try_from((u16::from(v) * a + 127) / 255).unwrap_or(255);
    [m(c[0]), m(c[1]), m(c[2]), c[3]]
}

/// Fill `content` with `vello_cpu` over white: premultiplied RGBA rows.
pub(crate) fn fill_vello(content: &Content) -> Result<Vec<u8>, String> {
    use vello_cpu::{Pixmap, RenderContext, Resources};
    let (width, height) = (
        u16::try_from(content.width).map_err(|e| e.to_string())?,
        u16::try_from(content.height).map_err(|e| e.to_string())?,
    );
    let mut renderer = RenderContext::new(width, height);
    renderer.set_paint(AlphaColor::<Srgb>::from_rgba8(255, 255, 255, 255));
    renderer.fill_rect(&KRect::new(0.0, 0.0, f64::from(width), f64::from(height)));
    for item in &content.items {
        let [red, green, blue, alpha] = item.color;
        renderer.set_paint(AlphaColor::<Srgb>::from_rgba8(red, green, blue, alpha));
        renderer.fill_path(&item.path);
    }
    renderer.flush();
    let mut pixmap = Pixmap::new(width, height);
    renderer.render(&mut pixmap, &mut Resources::new());
    Ok(pixmap
        .data()
        .iter()
        .flat_map(|p| [p.r, p.g, p.b, p.a])
        .collect())
}

/// Fill `content` with `zeno`, one mask per path, blended over white:
/// premultiplied RGBA rows.
pub(crate) fn fill_zeno(content: &Content) -> Vec<u8> {
    use zeno::{Command as Z, Mask, Point as P};
    let (width, height) = (content.width, content.height);
    let mut rgba = vec![255_u8; width * height * 4];
    let point = |p: Point| P::new(to_f32(p.x), to_f32(p.y));
    for item in &content.items {
        let commands: Vec<Z> = item
            .path
            .elements()
            .iter()
            .map(|e| match *e {
                PathEl::MoveTo(p) => Z::MoveTo(point(p)),
                PathEl::LineTo(p) => Z::LineTo(point(p)),
                PathEl::QuadTo(c, p) => Z::QuadTo(point(c), point(p)),
                PathEl::CurveTo(c0, c1, end) => Z::CurveTo(point(c0), point(c1), point(end)),
                PathEl::ClosePath => Z::Close,
            })
            .collect();
        let (mask, place) = Mask::new(&commands).render();
        let color = premultiply(item.color);
        for row in 0..place.height as usize {
            let y = i64::from(place.top) + i64::try_from(row).unwrap_or(0);
            let Ok(y) = usize::try_from(y) else { continue };
            if y >= height {
                continue;
            }
            for col in 0..place.width as usize {
                let x = i64::from(place.left) + i64::try_from(col).unwrap_or(0);
                let Ok(x) = usize::try_from(x) else { continue };
                if x >= width {
                    continue;
                }
                let coverage = u32::from(mask[row * place.width as usize + col]);
                if coverage == 0 {
                    continue;
                }
                let at = (y * width + x) * 4;
                let alpha = u32::from(color[3]) * coverage / 255;
                for c in 0..4 {
                    let src = u32::from(color[c]) * coverage / 255;
                    let dst = u32::from(rgba[at + c]);
                    rgba[at + c] = u8::try_from(src + dst * (255 - alpha) / 255).unwrap_or(255);
                }
            }
        }
    }
    rgba
}

/// The GPU's input: every path flattened into fan triangles, and a cover per
/// path.
#[derive(Clone, Debug, Default)]
pub(crate) struct Fans {
    pub(crate) vertices: Vec<[f32; 2]>,
    pub(crate) covers: Vec<Cover>,
    pub(crate) draws: Vec<PathDraw>,
}

/// Flatten `content` for the GPU (S6-Q4's CPU side of GPU filling).
pub(crate) fn fans(content: &Content) -> Fans {
    let mut fans = Fans::default();
    for item in &content.items {
        let first = fans.vertices.len();
        let mut contour: Vec<[f32; 2]> = Vec::new();
        let flush = |contour: &mut Vec<[f32; 2]>, out: &mut Vec<[f32; 2]>| {
            for k in 1..contour.len().saturating_sub(1) {
                out.push(contour[0]);
                out.push(contour[k]);
                out.push(contour[k + 1]);
            }
            contour.clear();
        };
        vello_cpu::kurbo::flatten(item.path.iter(), TOLERANCE, |el| match el {
            PathEl::MoveTo(p) => {
                flush(&mut contour, &mut fans.vertices);
                contour.push([to_f32(p.x), to_f32(p.y)]);
            }
            PathEl::LineTo(p) => contour.push([to_f32(p.x), to_f32(p.y)]),
            PathEl::ClosePath => flush(&mut contour, &mut fans.vertices),
            PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
        });
        flush(&mut contour, &mut fans.vertices);
        let b = item.path.bounding_box();
        let c = premultiply(item.color);
        fans.covers.push(Cover {
            rect: [
                to_f32(b.x0.floor()),
                to_f32(b.y0.floor()),
                to_f32(b.width().ceil() + 1.0),
                to_f32(b.height().ceil() + 1.0),
            ],
            color: c.map(|v| f32::from(v) / 255.0),
        });
        fans.draws.push(PathDraw {
            first_vertex: first,
            vertex_count: fans.vertices.len() - first,
            cover: fans.covers.len() - 1,
        });
    }
    fans
}

/// How far `other` is from `reference` (both premultiplied RGBA over white):
/// the mean absolute difference per channel, the largest, and the share of
/// pixels with any channel more than 32 apart.
pub(crate) fn compare(reference: &[u8], other: &[u8]) -> (f64, u8, f64) {
    let (mut sum, mut max, mut visible, mut pixels) = (0_u64, 0_u8, 0_u64, 0_u64);
    for (a, b) in reference.chunks_exact(4).zip(other.chunks_exact(4)) {
        let mut worst = 0;
        for c in 0..3 {
            let d = a[c].abs_diff(b[c]);
            sum += u64::from(d);
            worst = worst.max(d);
        }
        max = max.max(worst);
        if worst > 32 {
            visible += 1;
        }
        pixels += 1;
    }
    let n = to_f64_u64(pixels.max(1));
    (to_f64_u64(sum) / (n * 3.0), max, to_f64_u64(visible) / n)
}

fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[expect(clippy::cast_precision_loss, reason = "pixel counts far below 2^52")]
fn to_f64_u64(n: u64) -> f64 {
    n as f64
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "canvas coordinates are small"
)]
fn to_f32(v: f64) -> f32 {
    v as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contents_are_path_heavy_and_both_cpu_fillers_agree() {
        let all = contents();
        assert_eq!(all.len(), 3);
        for content in &all {
            assert!(
                content.items.len() > 10 && content.segments() > 400,
                "{}",
                content.name
            );
            let fans = fans(content);
            assert_eq!(fans.draws.len(), content.items.len());
            assert!(fans.vertices.len().is_multiple_of(3) && !fans.vertices.is_empty());
        }
        let icons = &all[0];
        let vello = fill_vello(icons).expect("vello_cpu fills");
        let zeno = fill_zeno(icons);
        let (mean, _, visible) = compare(&vello, &zeno);
        assert!(
            mean < 2.0 && visible < 0.01,
            "mean {mean}, visible {visible}"
        );
    }
}
