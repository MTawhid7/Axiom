//! S3-Q4 (C2): node layouts for S1's `traverse` suite. Does taking the
//! index arithmetic out of the dependent chain of loads speed up pointer
//! chasing over `u32` handles?
//!
//! In S1's Pattern B, each hop of a traversal loads a link, subtracts one
//! (handles store index + 1), bounds-checks, multiplies by the 104-byte node
//! size (`madd`), and only then loads the next field. Four layouts of the
//! same tree, in the same memory order, separate the factors:
//!
//! | Variant | Node | Indexing | Arithmetic between dependent loads |
//! |---|---|---|---|
//! | `aos104` | 104 bytes, S1's size | 0-based: `raw - 1` | subtract, multiply-add |
//! | `aos104-1b` | 104 bytes | 1-based: a dummy node 0, so `raw` indexes directly | multiply-add |
//! | `aos128-1b` | 128 bytes, a power of two | 1-based | one shifted add |
//! | `soa-1b` | `u32` link columns, struct-of-arrays | 1-based | none: the scaling is the load's own addressing mode |
//!
//! Every variant reads the same payload per node (id, kind code, attribute
//! count) and folds it as S1's `traverse` does, so equal folds prove equal
//! walks.

use s1_bench::dom::{Fold, kind};
use s1_scenarios::NodeId;
use s3_gc::{Barrier, RootStack};

use crate::dom::{NodeKind, NodeRef};
use crate::port::Port;

/// What the fold reads from a node.
#[derive(Clone, Copy, Debug, Default)]
struct Payload {
    /// The id plus one; 0 for none.
    id: u32,
    kind: u32,
    attributes: u32,
}

impl Payload {
    #[inline]
    fn visit(self, fold: &mut Fold) {
        let id = (self.id != 0).then(|| NodeId(self.id - 1));
        fold.visit(id, u64::from(self.kind), self.attributes as usize);
    }
}

/// A node of S1's size: links, payload, and padding to 104 bytes.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct Node104 {
    first_child: u32,
    next_sibling: u32,
    parent: u32,
    payload: Payload,
    _pad: [u32; 20],
}

/// The same node with a power-of-two stride.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(128))]
struct Node128 {
    first_child: u32,
    next_sibling: u32,
    parent: u32,
    payload: Payload,
    _pad: [u32; 20],
}

const _: () = assert!(size_of::<Node104>() == 104 && size_of::<Node128>() == 128);

/// One tree in every layout. Node `i` of the port's arena is element `i`
/// (0-based) or `i + 1` (1-based) everywhere, so memory order is preserved.
#[derive(Debug)]
pub struct Layouts {
    document: u32,
    aos0: Vec<Node104>,
    aos1: Vec<Node104>,
    p2: Vec<Node128>,
    soa_first_child: Vec<u32>,
    soa_next_sibling: Vec<u32>,
    soa_parent: Vec<u32>,
    soa_payload: Vec<Payload>,
}

/// The variants, in report order.
pub const VARIANTS: [&str; 4] = ["aos104", "aos104-1b", "aos128-1b", "soa-1b"];

fn raw(handle: Option<NodeRef<'static>>) -> u32 {
    handle.map_or(0, |h| u32::try_from(h.index() + 1).expect("u32 handles"))
}

impl Layouts {
    /// Copy the tree held by `port` into every layout.
    #[must_use]
    pub fn from_port<S: RootStack, B: Barrier>(port: &Port<'_, S, B>) -> Self {
        let agent = port.agent();
        let nodes = &agent.heap.nodes;
        let len = nodes.len();
        let empty = Node104 {
            first_child: 0,
            next_sibling: 0,
            parent: 0,
            payload: Payload::default(),
            _pad: [0; 20],
        };
        let mut aos1 = vec![empty; len + 1];
        let mut stack = vec![agent.globals.document];
        while let Some(node) = stack.pop() {
            let data = &nodes[node];
            let code = match &data.kind {
                NodeKind::Document => kind::DOCUMENT,
                NodeKind::Element(tag) => kind::element(*tag),
                NodeKind::Text(_) => kind::TEXT,
            };
            aos1[node.index() + 1] = Node104 {
                first_child: raw(data.first_child()),
                next_sibling: raw(data.next_sibling()),
                parent: raw(data.parent()),
                payload: Payload {
                    id: data.id.map_or(0, |id| id.0 + 1),
                    kind: u32::try_from(code).expect("small codes"),
                    attributes: u32::try_from(data.attributes.len()).expect("few"),
                },
                _pad: [0; 20],
            };
            let mut child = data.first_child();
            while let Some(c) = child {
                stack.push(c);
                child = nodes[c].next_sibling();
            }
        }
        let p2 = aos1
            .iter()
            .map(|n| Node128 {
                first_child: n.first_child,
                next_sibling: n.next_sibling,
                parent: n.parent,
                payload: n.payload,
                _pad: [0; 20],
            })
            .collect();
        Self {
            document: raw(Some(agent.globals.document)),
            aos0: aos1[1..].to_vec(),
            soa_first_child: aos1.iter().map(|n| n.first_child).collect(),
            soa_next_sibling: aos1.iter().map(|n| n.next_sibling).collect(),
            soa_parent: aos1.iter().map(|n| n.parent).collect(),
            soa_payload: aos1.iter().map(|n| n.payload).collect(),
            aos1,
            p2,
        }
    }

    /// Traverse in the named layout.
    ///
    /// # Panics
    /// If `variant` is not one of [`VARIANTS`].
    #[must_use]
    pub fn traverse(&self, variant: &str) -> Fold {
        match variant {
            "aos104" => self.traverse_aos0(),
            "aos104-1b" => self.traverse_aos1(),
            "aos128-1b" => self.traverse_p2(),
            "soa-1b" => self.traverse_soa(),
            _ => panic!("unknown layout {variant:?}"),
        }
    }

    // Each walk is S1's `traverse` loop (`s1_bench::pattern_b`), written out
    // per layout so that each compiles to its own addressing.

    fn traverse_aos0(&self) -> Fold {
        let nodes = &self.aos0[..];
        let mut fold = Fold::default();
        let mut node = self.document;
        loop {
            let mut data = &nodes[node as usize - 1];
            data.payload.visit(&mut fold);
            if data.first_child != 0 {
                node = data.first_child;
                continue;
            }
            loop {
                if node == self.document {
                    return fold;
                }
                if data.next_sibling != 0 {
                    node = data.next_sibling;
                    break;
                }
                node = data.parent;
                data = &nodes[node as usize - 1];
            }
        }
    }

    fn traverse_aos1(&self) -> Fold {
        let nodes = &self.aos1[..];
        let mut fold = Fold::default();
        let mut node = self.document;
        loop {
            let mut data = &nodes[node as usize];
            data.payload.visit(&mut fold);
            if data.first_child != 0 {
                node = data.first_child;
                continue;
            }
            loop {
                if node == self.document {
                    return fold;
                }
                if data.next_sibling != 0 {
                    node = data.next_sibling;
                    break;
                }
                node = data.parent;
                data = &nodes[node as usize];
            }
        }
    }

    fn traverse_p2(&self) -> Fold {
        let nodes = &self.p2[..];
        let mut fold = Fold::default();
        let mut node = self.document;
        loop {
            let mut data = &nodes[node as usize];
            data.payload.visit(&mut fold);
            if data.first_child != 0 {
                node = data.first_child;
                continue;
            }
            loop {
                if node == self.document {
                    return fold;
                }
                if data.next_sibling != 0 {
                    node = data.next_sibling;
                    break;
                }
                node = data.parent;
                data = &nodes[node as usize];
            }
        }
    }

    fn traverse_soa(&self) -> Fold {
        let (first_child, next_sibling, parent, payload) = (
            &self.soa_first_child[..],
            &self.soa_next_sibling[..],
            &self.soa_parent[..],
            &self.soa_payload[..],
        );
        let mut fold = Fold::default();
        let mut node = self.document;
        loop {
            payload[node as usize].visit(&mut fold);
            let child = first_child[node as usize];
            if child != 0 {
                node = child;
                continue;
            }
            loop {
                if node == self.document {
                    return fold;
                }
                let next = next_sibling[node as usize];
                if next != 0 {
                    node = next;
                    break;
                }
                node = parent[node as usize];
            }
        }
    }
}
