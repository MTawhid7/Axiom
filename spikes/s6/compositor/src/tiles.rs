//! Tiles: a layer cut into square tiles (GR2), damage mapped to the tiles it
//! touches (GR7), and, per tile, the items of each kind that touch it.

use crate::scene::List;

/// A layer's tiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Grid {
    /// The tile's side, in pixels.
    pub(crate) tile: usize,
    pub(crate) cols: usize,
    pub(crate) rows: usize,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl Grid {
    pub(crate) fn new(width: f32, height: f32, tile: usize) -> Self {
        let side = to_f32(tile);
        Self {
            tile,
            cols: to_count((width / side).ceil()),
            rows: to_count((height / side).ceil()),
            width,
            height,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.cols * self.rows
    }

    /// The tile at column `col`, row `row`: its index.
    pub(crate) fn index(&self, col: usize, row: usize) -> usize {
        row * self.cols + col
    }

    /// The tile `index`: its top-left corner, in layer pixels.
    pub(crate) fn origin(&self, index: usize) -> [f32; 2] {
        let side = to_f32(self.tile);
        [
            to_f32(index % self.cols) * side,
            to_f32(index / self.cols) * side,
        ]
    }

    /// The tiles `rect` (x, y, width, height) touches, in index order.
    pub(crate) fn touched(&self, rect: [f32; 4]) -> Vec<usize> {
        let side = to_f32(self.tile);
        let (x0, y0) = (rect[0].max(0.0), rect[1].max(0.0));
        let (x1, y1) = (
            (rect[0] + rect[2]).min(self.width),
            (rect[1] + rect[3]).min(self.height),
        );
        if x1 <= x0 || y1 <= y0 {
            return Vec::new();
        }
        let (c0, r0) = (to_count((x0 / side).floor()), to_count((y0 / side).floor()));
        let c1 = to_count(((x1 / side).ceil()).min(to_f32(self.cols)));
        let r1 = to_count(((y1 / side).ceil()).min(to_f32(self.rows)));
        let mut out = Vec::with_capacity((c1 - c0) * (r1 - r0));
        for row in r0..r1 {
            for col in c0..c1 {
                out.push(self.index(col, row));
            }
        }
        out
    }

    /// The tiles between layer rows `top` and `bottom` (a display port).
    pub(crate) fn rows_between(&self, top: f32, bottom: f32) -> Vec<usize> {
        self.touched([0.0, top, self.width, bottom - top])
    }
}

/// For one kind of item: every tile's item indices, concatenated, and each
/// tile's range in that list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Bins {
    pub(crate) ids: Vec<u32>,
    /// Per tile: the first index and the count.
    pub(crate) ranges: Vec<(usize, usize)>,
}

impl Bins {
    /// Bin items by the boxes `boxes` (x, y, width, height), already grown
    /// by whatever the items draw outside their rectangles.
    pub(crate) fn new(grid: &Grid, boxes: impl Iterator<Item = [f32; 4]>) -> Self {
        let mut per_tile: Vec<Vec<u32>> = vec![Vec::new(); grid.count()];
        for (item, b) in boxes.enumerate() {
            let id = u32::try_from(item).unwrap_or(u32::MAX);
            for tile in grid.touched(b) {
                per_tile[tile].push(id);
            }
        }
        let mut bins = Self::default();
        for ids in per_tile {
            bins.ranges.push((bins.ids.len(), ids.len()));
            bins.ids.extend(ids);
        }
        bins
    }
}

/// The bins of each kind for a list, in drawing order: opaque rectangles,
/// shadows, gradients, rectangles, images, glyphs.
pub(crate) fn bin_list(grid: &Grid, list: &List) -> [Bins; 6] {
    let grow = |r: [f32; 4], by: f32| [r[0] - by, r[1] - by, r[2] + 2.0 * by, r[3] + 2.0 * by];
    [
        Bins::new(grid, list.opaque.iter().map(|r| r.bounds)),
        Bins::new(
            grid,
            list.shadows
                .iter()
                .map(|s| grow(s.rect, 3.0 * s.params[1] + 1.0)),
        ),
        Bins::new(grid, list.gradients.iter().map(|g| grow(g.rect, 1.0))),
        Bins::new(
            grid,
            list.rects.iter().map(|r| {
                if r.extra[1] == 0.0 {
                    grow(r.bounds, 1.0)
                } else {
                    // A rotated item: its circumscribed square.
                    let half = (r.bounds[2].hypot(r.bounds[3])) * 0.5 + 1.0;
                    let (cx, cy) = (
                        r.bounds[0] + r.bounds[2] * 0.5,
                        r.bounds[1] + r.bounds[3] * 0.5,
                    );
                    [cx - half, cy - half, 2.0 * half, 2.0 * half]
                }
            }),
        ),
        Bins::new(grid, list.images.iter().map(|m| m.rect)),
        Bins::new(grid, list.glyphs.iter().map(|g| g.rect)),
    ]
}

pub(crate) fn to_f32(n: usize) -> f32 {
    f32::from(u16::try_from(n).unwrap_or(u16::MAX))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "tile counts are small and positive"
)]
fn to_count(v: f32) -> usize {
    v.max(0.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_touch_the_tiles_they_overlap() {
        let grid = Grid::new(1000.0, 600.0, 256);
        assert_eq!((grid.cols, grid.rows), (4, 3));
        assert_eq!(grid.touched([10.0, 10.0, 20.0, 20.0]), vec![0]);
        assert_eq!(grid.touched([250.0, 250.0, 10.0, 10.0]), vec![0, 1, 4, 5]);
        assert!(grid.touched([2000.0, 0.0, 10.0, 10.0]).is_empty());
        let [x, y] = grid.origin(5);
        assert!((x - 256.0).abs() < 1e-6 && (y - 256.0).abs() < 1e-6);
        let bins = Bins::new(
            &grid,
            [[10.0, 10.0, 600.0, 20.0], [700.0, 300.0, 10.0, 10.0]].into_iter(),
        );
        assert_eq!(bins.ranges.len(), 12);
        let tile = |i: usize| &bins.ids[bins.ranges[i].0..bins.ranges[i].0 + bins.ranges[i].1];
        assert_eq!(tile(0), &[0]);
        assert_eq!(tile(2), &[0]);
        assert_eq!(tile(6), &[1]);
        assert!(tile(11).is_empty());
    }
}
