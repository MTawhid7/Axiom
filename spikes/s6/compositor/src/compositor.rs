//! The compositor: display lists into tiled layers (GR2), redrawing only the
//! tiles that damage touches (GR7), and scrolling and animating by moving
//! layers (GR8), all on its own thread (GR11).

use std::collections::{HashMap, HashSet};

use crate::platform::gpu::{Buffer, Draw, Glyph, Gpu, GpuTime, Indices, Kind, Pod, Rect, Texture};
use crate::platform::layers::{self, Layer};
use crate::platform::surface::{Surface, Tag};
use crate::scene::List;
use crate::tiles::{Bins, Grid, bin_list};

/// The kinds, in drawing order (the order of [`bin_list`]).
const KINDS: [Kind; 6] = [
    Kind::RectOpaque,
    Kind::Shadow,
    Kind::Gradient,
    Kind::Rect,
    Kind::Image,
    Kind::Glyph,
];

/// The textures that items read.
#[derive(Debug)]
pub(crate) struct Atlases {
    pub(crate) glyphs: Texture,
    pub(crate) images: Texture,
}

/// One kind of item on the GPU.
#[derive(Debug)]
struct KindData {
    instances: Buffer,
    count: usize,
    ids: Indices,
    /// Per tile: the first index into `ids`, and the count.
    ranges: Vec<(usize, usize)>,
}

/// A tile: its layer, the surface it shows, and a spare to draw the next
/// version into.
#[derive(Debug)]
struct Tile {
    layer: Layer,
    front: Option<Surface>,
    spare: Option<Surface>,
}

/// What one redraw did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Redraw {
    pub(crate) tiles: usize,
    pub(crate) gpu: GpuTime,
    /// Surfaces newly allocated, because no spare was free.
    pub(crate) allocated: usize,
}

/// A layer drawn in tiles.
#[derive(Debug)]
pub(crate) struct Tiled {
    pub(crate) list: List,
    pub(crate) grid: Grid,
    kinds: Vec<KindData>,
    /// The layer the tiles sit in; it moves to scroll, and transforms to
    /// animate.
    pub(crate) container: Layer,
    tiles: HashMap<usize, Tile>,
    /// Surfaces of tiles that left the display port, for new tiles to reuse,
    /// as Firefox's surface pool does.
    pool: Vec<Surface>,
    tag: Tag,
    scale: f64,
}

/// How many free surfaces a layer keeps: two rows of a 2,560-pixel layer in
/// 256-pixel tiles.
const POOL_LIMIT: usize = 20;

impl Tiled {
    /// Upload `list` and bin its items into tiles of `tile` pixels.
    pub(crate) fn new(
        gpu: &Gpu,
        list: List,
        tile: usize,
        scale: f64,
        tag: Tag,
    ) -> Result<Self, String> {
        let grid = Grid::new(list.width, list.height, tile);
        let bins = bin_list(&grid, &list);
        let mut kinds = Vec::with_capacity(KINDS.len());
        for (kind, bins) in KINDS.iter().zip(bins) {
            let (instances, count) = match kind {
                Kind::RectOpaque => upload(gpu, &list.opaque)?,
                Kind::Shadow => upload(gpu, &list.shadows)?,
                Kind::Gradient => upload(gpu, &list.gradients)?,
                Kind::Rect => upload(gpu, &list.rects)?,
                Kind::Image => upload(gpu, &list.images)?,
                Kind::Glyph => upload(gpu, &list.glyphs)?,
            };
            let Bins { ids, ranges } = bins;
            kinds.push(KindData {
                instances,
                count,
                ids: gpu.indices(&ids, count)?,
                ranges,
            });
        }
        let container = Layer::new(scale);
        container.set_frame(
            0.0,
            0.0,
            f64::from(list.width) / scale,
            f64::from(list.height) / scale,
        );
        container.set_clips(true);
        Ok(Self {
            list,
            grid,
            kinds,
            container,
            tiles: HashMap::new(),
            pool: Vec::new(),
            tag,
            scale,
        })
    }

    /// How many tiles exist.
    pub(crate) fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// The bytes IOSurface allocated for the tiles, spares and pool included.
    pub(crate) fn tile_bytes(&self) -> usize {
        let tiles: usize = self
            .tiles
            .values()
            .map(|t| {
                t.front.as_ref().map_or(0, Surface::bytes)
                    + t.spare.as_ref().map_or(0, Surface::bytes)
            })
            .sum();
        tiles + self.pool.iter().map(Surface::bytes).sum::<usize>()
    }

    /// Free every spare and pooled surface, keeping only what is shown.
    pub(crate) fn trim(&mut self) {
        self.pool.clear();
        for tile in self.tiles.values_mut() {
            tile.spare = None;
        }
    }

    /// Draw `tiles`, creating any that don't exist yet, then show them all in
    /// one transaction.
    pub(crate) fn redraw(
        &mut self,
        gpu: &Gpu,
        atlases: &Atlases,
        tiles: &[usize],
    ) -> Result<Redraw, String> {
        let mut recording = gpu.record()?;
        let mut finished = Vec::with_capacity(tiles.len());
        let mut allocated = 0;
        let background = self.list.background.map(f64::from);
        for &index in tiles {
            if index >= self.grid.count() {
                return Err(format!("tile {index} is outside the grid"));
            }
            let reusable = self
                .tiles
                .get_mut(&index)
                .and_then(|t| t.spare.take_if(|s| !s.in_use()));
            let surface = if let Some(surface) = reusable {
                surface
            } else if let Some(free) = self.pool.iter().position(|s| !s.in_use()) {
                self.pool.swap_remove(free)
            } else {
                allocated += 1;
                Surface::new(gpu.device(), self.grid.tile, self.grid.tile, self.tag)?
            };
            let draws = self.draws(index, atlases);
            recording.tile(
                surface.texture(),
                self.grid.origin(index),
                background,
                &draws,
            )?;
            finished.push((index, surface));
        }
        let gpu_time = recording.finish();
        let count = finished.len();
        let (scale, side) = (self.scale, to_f64(self.grid.tile) / self.scale);
        let grid = self.grid;
        let tiles_map = &mut self.tiles;
        let container = &self.container;
        layers::transaction(|| {
            for (index, surface) in finished {
                let tile = tiles_map.entry(index).or_insert_with(|| {
                    let layer = Layer::new(scale);
                    let [x, y] = grid.origin(index);
                    layer.set_frame(f64::from(x) / scale, f64::from(y) / scale, side, side);
                    layer.set_opaque(true);
                    container.add(&layer);
                    Tile {
                        layer,
                        front: None,
                        spare: None,
                    }
                });
                tile.layer.set_contents(&surface);
                tile.spare = tile.front.replace(surface);
            }
        });
        Ok(Redraw {
            tiles: count,
            gpu: gpu_time,
            allocated,
        })
    }

    /// Remove the tiles not in `keep`, pooling their surfaces up to
    /// [`POOL_LIMIT`] and freeing the rest.
    pub(crate) fn retain(&mut self, keep: &HashSet<usize>) -> usize {
        let gone: Vec<usize> = self
            .tiles
            .keys()
            .filter(|i| !keep.contains(i))
            .copied()
            .collect();
        let tiles_map = &mut self.tiles;
        let mut freed = Vec::new();
        layers::transaction(|| {
            for index in &gone {
                if let Some(tile) = tiles_map.remove(index) {
                    tile.layer.clear_contents();
                    tile.layer.remove();
                    freed.extend(tile.front);
                    freed.extend(tile.spare);
                }
            }
        });
        for surface in freed {
            if self.pool.len() < POOL_LIMIT {
                self.pool.push(surface);
            }
        }
        gone.len()
    }

    /// The tiles that exist.
    pub(crate) fn existing(&self) -> HashSet<usize> {
        self.tiles.keys().copied().collect()
    }

    /// Rewrite rectangle `index`, here and on the GPU. Call only between
    /// frames.
    pub(crate) fn set_rect(&mut self, index: usize, rect: Rect) -> Result<(), String> {
        *self.list.rects.get_mut(index).ok_or("no such rectangle")? = rect;
        self.kinds[3].instances.write(index, &[rect])
    }

    /// Rewrite glyphs from `first`, here and on the GPU. Call only between
    /// frames.
    pub(crate) fn set_glyphs(&mut self, first: usize, glyphs: &[Glyph]) -> Result<(), String> {
        let slots = self
            .list
            .glyphs
            .get_mut(first..first + glyphs.len())
            .ok_or("glyph slots out of range")?;
        slots.copy_from_slice(glyphs);
        self.kinds[5].instances.write(first, glyphs)
    }

    fn draws<'a>(&'a self, index: usize, atlases: &'a Atlases) -> Vec<Draw<'a>> {
        KINDS
            .iter()
            .zip(&self.kinds)
            .map(|(&kind, data)| {
                let (first, count) = data.ranges[index];
                Draw {
                    kind,
                    instances: &data.instances,
                    instance_count: data.count,
                    ids: &data.ids,
                    first,
                    count,
                    texture: match kind {
                        Kind::Image => Some(&atlases.images),
                        Kind::Glyph => Some(&atlases.glyphs),
                        _ => None,
                    },
                }
            })
            .collect()
    }
}

fn upload<T: Pod>(gpu: &Gpu, items: &[T]) -> Result<(Buffer, usize), String> {
    Ok((gpu.upload(items)?, items.len()))
}

pub(crate) fn to_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}
