//! The display-list update a renderer sends the GPU process, and the GPU
//! process's decoder and validator: PL9's checks in safe Rust. Decoding copies
//! every value out of the received bytes once, then checks the copy (IR7).
//!
//! The format is a stand-in for the generated one (IP8): little-endian, a
//! header, then fixed-size property-tree nodes, items, the resources the
//! update uses, and the tiles it damages.

use std::fmt;

pub(crate) const MAGIC: u32 = u32::from_le_bytes(*b"S10D");
pub(crate) const VERSION: u16 = 1;
const HEADER: usize = 32;
const NODE: usize = 32;
const ITEM: usize = 48;

pub(crate) const MAX_NODES: usize = 1 << 16;
pub(crate) const MAX_ITEMS: usize = 1 << 20;
pub(crate) const MAX_RESOURCES: usize = 1 << 12;
/// The largest coordinate or size a list may carry, in pixels.
pub(crate) const MAX_COORDINATE: f32 = 1.0e7;
/// The root node's parent.
pub(crate) const NO_PARENT: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// A 2D affine transform, `[a, b, c, d, tx, ty]`.
    Transform,
    /// A clip rectangle, `[x, y, width, height]`.
    Clip,
    /// An effect, `[opacity]`.
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Node {
    pub(crate) parent: u32,
    pub(crate) kind: NodeKind,
    pub(crate) data: [f32; 6],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ItemKind {
    Rect,
    Image,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Item {
    pub(crate) kind: ItemKind,
    /// The property-tree node it draws in.
    pub(crate) node: u32,
    /// `[x, y, width, height]` in the node's space, in pixels.
    pub(crate) rect: [f32; 4],
    /// RGBA, 8 bits each, premultiplied.
    pub(crate) color: u32,
    /// For an image: the resource it samples.
    pub(crate) resource: u32,
    /// For an image: the normalized source rectangle.
    pub(crate) src: [f32; 4],
}

/// A decoded, validated update.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DisplayList {
    pub(crate) frame: u64,
    pub(crate) nodes: Vec<Node>,
    pub(crate) items: Vec<Item>,
    pub(crate) resources: Vec<u32>,
    pub(crate) damage: Vec<u32>,
}

/// Why an update was refused: always `Terminate` for its sender (IP11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Violation {
    Truncated,
    Trailing,
    BadMagic,
    BadVersion,
    TooMany,
    BadParent,
    BadKind,
    NotFinite,
    OutOfRange,
    UnknownResource,
    BadDamage,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// What the GPU process knows about the sender: its registered resources,
/// sorted by ID, and the number of tiles.
#[derive(Debug)]
pub(crate) struct Context<'a> {
    pub(crate) registered: &'a [u32],
    pub(crate) tiles: u32,
}

impl DisplayList {
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&self.frame.to_le_bytes());
        for n in [
            self.nodes.len(),
            self.items.len(),
            self.resources.len(),
            self.damage.len(),
        ] {
            out.extend_from_slice(&count(n).to_le_bytes());
        }
        for node in &self.nodes {
            out.extend_from_slice(&node.parent.to_le_bytes());
            let kind: u32 = match node.kind {
                NodeKind::Transform => 0,
                NodeKind::Clip => 1,
                NodeKind::Effect => 2,
            };
            out.extend_from_slice(&kind.to_le_bytes());
            for value in node.data {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for item in &self.items {
            let kind: u32 = match item.kind {
                ItemKind::Rect => 0,
                ItemKind::Image => 1,
            };
            out.extend_from_slice(&kind.to_le_bytes());
            out.extend_from_slice(&item.node.to_le_bytes());
            for value in item.rect {
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&item.color.to_le_bytes());
            out.extend_from_slice(&item.resource.to_le_bytes());
            for value in item.src {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        for id in self.resources.iter().chain(&self.damage) {
            out.extend_from_slice(&id.to_le_bytes());
        }
    }

    /// Each node's accumulated translation (the spike's transforms translate
    /// only), so items can be placed in page pixels.
    pub(crate) fn translations(&self) -> Vec<[f32; 2]> {
        let mut out: Vec<[f32; 2]> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let base = if node.parent == NO_PARENT {
                [0.0, 0.0]
            } else {
                out[node.parent as usize]
            };
            out.push(match node.kind {
                NodeKind::Transform => [base[0] + node.data[4], base[1] + node.data[5]],
                NodeKind::Clip | NodeKind::Effect => base,
            });
        }
        out
    }
}

/// Reads little-endian values from a byte slice, never past its end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Violation> {
        let end = self.at.checked_add(N).ok_or(Violation::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(Violation::Truncated)?;
        self.at = end;
        let mut out = [0; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    fn u16(&mut self) -> Result<u16, Violation> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    fn u32(&mut self) -> Result<u32, Violation> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, Violation> {
        Ok(u64::from_le_bytes(self.take()?))
    }

    /// A finite `f32` whose magnitude is at most [`MAX_COORDINATE`].
    fn coordinate(&mut self) -> Result<f32, Violation> {
        let value = f32::from_le_bytes(self.take()?);
        if !value.is_finite() {
            return Err(Violation::NotFinite);
        }
        if value.abs() > MAX_COORDINATE {
            return Err(Violation::OutOfRange);
        }
        Ok(value)
    }

    fn count(&mut self, limit: usize) -> Result<usize, Violation> {
        let n = self.u32()? as usize;
        if n > limit {
            Err(Violation::TooMany)
        } else {
            Ok(n)
        }
    }
}

/// Decode and validate `bytes` against the sender's `context`: every count
/// within its limit and the length exact; every node's parent earlier in the
/// list, so the tree is acyclic; every number finite and in range; every item
/// in an existing node; every image's resource registered by this sender and
/// listed by the update; every damaged tile real, with none listed twice.
pub(crate) fn decode(bytes: &[u8], context: &Context<'_>) -> Result<DisplayList, Violation> {
    let mut r = Reader { bytes, at: 0 };
    if r.u32()? != MAGIC {
        return Err(Violation::BadMagic);
    }
    if r.u16()? != VERSION {
        return Err(Violation::BadVersion);
    }
    let _flags = r.u16()?;
    let frame = r.u64()?;
    let node_count = r.count(MAX_NODES)?;
    let item_count = r.count(MAX_ITEMS)?;
    let resource_count = r.count(MAX_RESOURCES)?;
    let damage_count = r.count(context.tiles as usize)?;
    let expected =
        HEADER + node_count * NODE + item_count * ITEM + (resource_count + damage_count) * 4;
    if bytes.len() < expected {
        return Err(Violation::Truncated);
    }
    if bytes.len() > expected {
        return Err(Violation::Trailing);
    }

    let nodes = decode_nodes(&mut r, node_count)?;
    let items = decode_items(&mut r, item_count, node_count)?;

    let mut resources = Vec::with_capacity(resource_count);
    for _ in 0..resource_count {
        let id = r.u32()?;
        if context.registered.binary_search(&id).is_err() {
            return Err(Violation::UnknownResource);
        }
        resources.push(id);
    }
    resources.sort_unstable();
    for item in &items {
        if item.kind == ItemKind::Image && resources.binary_search(&item.resource).is_err() {
            return Err(Violation::UnknownResource);
        }
    }

    let mut damage = Vec::with_capacity(damage_count);
    for _ in 0..damage_count {
        let tile = r.u32()?;
        if tile >= context.tiles || damage.last().is_some_and(|&last| tile <= last) {
            return Err(Violation::BadDamage);
        }
        damage.push(tile);
    }
    Ok(DisplayList {
        frame,
        nodes,
        items,
        resources,
        damage,
    })
}

/// The property tree: each node's parent earlier in the list, so it is
/// acyclic, and each node's data in range for its kind.
fn decode_nodes(r: &mut Reader<'_>, count: usize) -> Result<Vec<Node>, Violation> {
    let mut nodes = Vec::with_capacity(count);
    for index in 0..count {
        let parent = r.u32()?;
        if parent != NO_PARENT && parent as usize >= index {
            return Err(Violation::BadParent);
        }
        let kind = match r.u32()? {
            0 => NodeKind::Transform,
            1 => NodeKind::Clip,
            2 => NodeKind::Effect,
            _ => return Err(Violation::BadKind),
        };
        let mut data = [0.0; 6];
        for value in &mut data {
            *value = r.coordinate()?;
        }
        let valid = match kind {
            NodeKind::Transform => true,
            NodeKind::Clip => data[2] >= 0.0 && data[3] >= 0.0,
            NodeKind::Effect => (0.0..=1.0).contains(&data[0]),
        };
        if !valid {
            return Err(Violation::OutOfRange);
        }
        nodes.push(Node { parent, kind, data });
    }
    Ok(nodes)
}

/// The items: each in an existing node, with a finite, non-negative
/// rectangle, and an image's source inside its resource.
fn decode_items(
    r: &mut Reader<'_>,
    count: usize,
    node_count: usize,
) -> Result<Vec<Item>, Violation> {
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = match r.u32()? {
            0 => ItemKind::Rect,
            1 => ItemKind::Image,
            _ => return Err(Violation::BadKind),
        };
        let node = r.u32()?;
        if node as usize >= node_count {
            return Err(Violation::BadParent);
        }
        let rect = [
            r.coordinate()?,
            r.coordinate()?,
            r.coordinate()?,
            r.coordinate()?,
        ];
        if rect[2] < 0.0 || rect[3] < 0.0 {
            return Err(Violation::OutOfRange);
        }
        let color = r.u32()?;
        let resource = r.u32()?;
        let src = [
            r.coordinate()?,
            r.coordinate()?,
            r.coordinate()?,
            r.coordinate()?,
        ];
        if kind == ItemKind::Image && !src.iter().all(|v| (0.0..=1.0).contains(v)) {
            return Err(Violation::OutOfRange);
        }
        items.push(Item {
            kind,
            node,
            rect,
            color,
            resource,
            src,
        });
    }
    Ok(items)
}

/// The viewport, in tiles.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Grid {
    pub(crate) columns: u32,
    pub(crate) rows: u32,
    pub(crate) side: u32,
}

impl Grid {
    pub(crate) fn tiles(self) -> u32 {
        self.columns * self.rows
    }

    /// The tile's top-left corner, in page pixels.
    pub(crate) fn origin(self, tile: u32) -> [f32; 2] {
        let (column, row) = (tile % self.columns, tile / self.columns);
        [to_f32(column * self.side), to_f32(row * self.side)]
    }
}

#[allow(clippy::cast_precision_loss)] // page coordinates stay far below 2^24
fn to_f32(n: u32) -> f32 {
    n as f32
}

/// A page-shaped list: words of text as small rectangles in lines down the
/// page, with an image every fiftieth item, under a small property tree (a
/// root, a scroll node and six effects), as S6's page was (44,372 items for a
/// 24,000-pixel page).
pub(crate) fn page(items: usize, width: f32, image: Option<u32>) -> DisplayList {
    let mut nodes = vec![
        Node {
            parent: NO_PARENT,
            kind: NodeKind::Transform,
            data: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        },
        Node {
            parent: 0,
            kind: NodeKind::Transform,
            data: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        },
    ];
    for _ in 0..6 {
        nodes.push(Node {
            parent: 1,
            kind: NodeKind::Effect,
            data: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        });
    }
    let (word, line, gap) = ([44.0_f32, 26.0_f32], 34.0_f32, 8.0_f32);
    let (mut x, mut y) = (gap, gap);
    let mut list = Vec::with_capacity(items);
    for i in 0..items {
        if x + word[0] > width {
            x = gap;
            y += line;
        }
        let index = u32::try_from(i).unwrap_or(u32::MAX);
        let node = 1 + index % 7;
        let item = match image {
            Some(resource) if i % 50 == 49 => Item {
                kind: ItemKind::Image,
                node,
                rect: [x, y, word[0], word[1]],
                color: 0,
                resource,
                src: [0.0, 0.0, 1.0, 1.0],
            },
            _ => Item {
                kind: ItemKind::Rect,
                node,
                rect: [x, y, word[0], word[1]],
                color: 0xff00_0000 | (index.wrapping_mul(2_654_435_761) & 0x007f_7f7f),
                resource: 0,
                src: [0.0; 4],
            },
        };
        list.push(item);
        x += word[0] + gap;
    }
    DisplayList {
        frame: 0,
        nodes,
        items: list,
        resources: image.into_iter().collect(),
        damage: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> (Vec<u32>, u32) {
        (vec![7], 70)
    }

    #[test]
    fn a_page_round_trips() {
        let mut page = page(1_000, 2560.0, Some(7));
        page.damage = vec![3, 4, 13, 14];
        let mut bytes = Vec::new();
        page.encode(&mut bytes);
        let (registered, tiles) = context();
        let decoded = decode(
            &bytes,
            &Context {
                registered: &registered,
                tiles,
            },
        )
        .unwrap();
        assert_eq!(decoded, page);
    }

    #[test]
    fn violations_are_reported_not_panicked() {
        let page = page(200, 2560.0, Some(7));
        let mut bytes = Vec::new();
        page.encode(&mut bytes);
        let (registered, tiles) = context();
        let context = Context {
            registered: &registered,
            tiles,
        };
        // Every truncation, and every single-byte change, either decodes or is
        // refused with a violation; none panics.
        for len in 0..bytes.len() {
            assert!(decode(&bytes[..len], &context).is_err());
        }
        let mut refused = 0;
        for at in 0..bytes.len() {
            let mut changed = bytes.clone();
            changed[at] ^= 0xa5;
            refused += usize::from(decode(&changed, &context).is_err());
        }
        assert!(refused > 0);
    }

    #[test]
    fn an_unregistered_resource_is_refused() {
        let page = page(100, 2560.0, Some(9));
        let mut bytes = Vec::new();
        page.encode(&mut bytes);
        let (registered, tiles) = context();
        let result = decode(
            &bytes,
            &Context {
                registered: &registered,
                tiles,
            },
        );
        assert_eq!(result, Err(Violation::UnknownResource));
    }

    #[test]
    fn a_cycle_is_refused() {
        let mut page = page(10, 2560.0, None);
        page.nodes[2].parent = 5;
        let mut bytes = Vec::new();
        page.encode(&mut bytes);
        let (registered, tiles) = context();
        let result = decode(
            &bytes,
            &Context {
                registered: &registered,
                tiles,
            },
        );
        assert_eq!(result, Err(Violation::BadParent));
    }
}
