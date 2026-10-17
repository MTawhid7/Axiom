//! Seeded workload generators.
//!
//! Each generator drives a [`Reference`] model while it records ops. That gives
//! three properties:
//! - every generated script is valid by construction, even though listener
//!   behaviours change the tree in the middle of a dispatch;
//! - the expected [`Outcome`] comes out alongside the script, at no extra cost;
//! - generation is fully deterministic: the same parameters and seed always
//!   produce the same script.
//!
//! | Workload | Stresses |
//! |---|---|
//! | [`tree`] | Allocation throughput; realistic depth and fan-out; attributes; traversal |
//! | [`listeners`] | Many listener closures with self and cross captures; dispatch semantics |
//! | [`churn`] | Random mutation; ids forgotten while subtrees are detached, producing garbage cycles |
//! | [`pressure`] | Long dispatches whose listeners allocate garbage, so collections must happen *mid-dispatch* |

use std::collections::VecDeque;

use crate::outcome::Outcome;
use crate::reference::{Reference, Stats};
use crate::rng::Rng;
use crate::script::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, NodeId, Script, ScriptBuilder, Tag,
    WeakId,
};

/// A generated script, plus the reference model's expected results for it.
#[derive(Clone, Debug)]
pub struct Generated {
    pub script: Script,
    pub expected: Outcome,
    pub stats: Stats,
}

/// Workload scale.
///
/// | Size | Intended use |
/// |---|---|
/// | `Small` | Fast enough for Miri (tens of nodes) |
/// | `Medium` | Unit and integration tests |
/// | `Large` | Benchmarks |
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
}

/// The S1 workloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Workload {
    Tree,
    Listeners,
    Churn,
    Pressure,
}

impl Workload {
    pub const ALL: [Self; 4] = [Self::Tree, Self::Listeners, Self::Churn, Self::Pressure];

    /// A short, stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Listeners => "listeners",
            Self::Churn => "churn",
            Self::Pressure => "pressure",
        }
    }

    /// Generate this workload at a preset `size`.
    #[must_use]
    pub fn generate(self, size: Size, seed: u64) -> Generated {
        match self {
            Self::Tree => tree(&TreeParams::preset(size), seed),
            Self::Listeners => listeners(&ListenersParams::preset(size), seed),
            Self::Churn => churn(&ChurnParams::preset(size), seed),
            Self::Pressure => pressure(&PressureParams::preset(size), seed),
        }
    }
}

// ----- Parameters -----

/// Shape of a generated document tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeParams {
    /// Number of elements, including `html` and `body`. Text nodes are extra.
    pub elements: u32,
    /// Depth at which a subtree's remaining budget becomes leaves. Elements
    /// sit at most `max_depth + 1` levels below the document.
    pub max_depth: u32,
    /// Chance, in thousandths, that an element gets a text child.
    pub text_per_mille: u32,
    /// Maximum attributes per element (capped at the 8 attribute names).
    pub max_attributes: u32,
    /// Whether to forget every id at the end, so that only the tree keeps the
    /// nodes alive.
    pub forget_ids: bool,
}

impl TreeParams {
    #[must_use]
    pub const fn preset(size: Size) -> Self {
        match size {
            Size::Small => Self {
                elements: 24,
                max_depth: 5,
                text_per_mille: 300,
                max_attributes: 3,
                forget_ids: true,
            },
            Size::Medium => Self {
                elements: 4_000,
                max_depth: 14,
                text_per_mille: 300,
                max_attributes: 4,
                forget_ids: true,
            },
            Size::Large => Self {
                elements: 100_000,
                max_depth: 24,
                text_per_mille: 300,
                max_attributes: 4,
                forget_ids: true,
            },
        }
    }
}

/// How randomly generated listeners are distributed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerMix {
    /// Chance, in thousandths, that a listener goes on the document.
    ///
    /// Every dispatch path ends at the document, so this dominates the
    /// invocation count at scale. Real pages put a few delegation listeners
    /// there, not thousands.
    pub document_per_mille: u32,
    /// Chance, in thousandths, that a closure captures a second, random node.
    pub extra_capture_per_mille: u32,
    /// Chance, in thousandths, of adding a removal pair (an earlier listener
    /// that removes the next one) instead of a single listener.
    pub removal_pair_per_mille: u32,
}

impl ListenerMix {
    /// The mix used by [`churn`].
    pub const CHURN: Self = Self {
        document_per_mille: 20,
        extra_capture_per_mille: 300,
        removal_pair_per_mille: 50,
    };
}

/// A listener-heavy document with many dispatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenersParams {
    /// The document (its `forget_ids` is ignored: ids are kept until the end).
    pub tree: TreeParams,
    pub listeners: u32,
    pub mix: ListenerMix,
    pub dispatches: u32,
    /// Checkpoints, spread evenly over the dispatches; one more follows at the end.
    pub checkpoints: u32,
}

impl ListenersParams {
    #[must_use]
    pub const fn preset(size: Size) -> Self {
        match size {
            Size::Small => Self {
                tree: TreeParams {
                    elements: 16,
                    max_depth: 5,
                    text_per_mille: 200,
                    max_attributes: 2,
                    forget_ids: false,
                },
                listeners: 24,
                mix: ListenerMix {
                    document_per_mille: 50,
                    extra_capture_per_mille: 250,
                    removal_pair_per_mille: 100,
                },
                dispatches: 24,
                checkpoints: 3,
            },
            Size::Medium => Self {
                tree: TreeParams {
                    elements: 1_000,
                    max_depth: 12,
                    text_per_mille: 200,
                    max_attributes: 3,
                    forget_ids: false,
                },
                listeners: 3_000,
                mix: ListenerMix {
                    document_per_mille: 10,
                    extra_capture_per_mille: 250,
                    removal_pair_per_mille: 50,
                },
                dispatches: 3_000,
                checkpoints: 10,
            },
            Size::Large => Self {
                tree: TreeParams {
                    elements: 20_000,
                    max_depth: 20,
                    text_per_mille: 200,
                    max_attributes: 3,
                    forget_ids: false,
                },
                listeners: 60_000,
                mix: ListenerMix {
                    document_per_mille: 2,
                    extra_capture_per_mille: 250,
                    removal_pair_per_mille: 50,
                },
                dispatches: 50_000,
                checkpoints: 20,
            },
        }
    }
}

/// A random mix of mutations, dispatches and forgets over a bounded pool of ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChurnParams {
    pub steps: u32,
    /// The pool of live (unforgotten) ids is kept at or below this size.
    pub target_live_ids: u32,
    pub collect_every: u32,
    pub checkpoint_every: u32,
}

impl ChurnParams {
    #[must_use]
    pub const fn preset(size: Size) -> Self {
        match size {
            Size::Small => Self {
                steps: 150,
                target_live_ids: 24,
                collect_every: 23,
                checkpoint_every: 50,
            },
            Size::Medium => Self {
                steps: 20_000,
                target_live_ids: 1_500,
                collect_every: 499,
                checkpoint_every: 2_000,
            },
            Size::Large => Self {
                steps: 300_000,
                target_live_ids: 20_000,
                collect_every: 4_999,
                checkpoint_every: 25_000,
            },
        }
    }
}

/// Long dispatches through a chain of nodes whose listeners all allocate
/// garbage.
///
/// **Invariant:** the reachable set is identical before and after the dispatch
/// phase (the two checkpoints match), and garbage totals
/// `dispatches × path_depth × listeners_per_node × garbage_per_invocation`.
/// A heap that collects mid-dispatch keeps its peak near the live set. One
/// that cannot collect mid-dispatch peaks near the live set plus *all* the
/// garbage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PressureParams {
    /// Length of the `div` chain from `body` to the target.
    pub path_depth: u32,
    /// Extra live elements hung off the chain.
    pub bystanders: u32,
    pub listeners_per_node: u32,
    pub garbage_per_invocation: u16,
    pub dispatches: u32,
}

impl PressureParams {
    #[must_use]
    pub const fn preset(size: Size) -> Self {
        match size {
            Size::Small => Self {
                path_depth: 4,
                bystanders: 6,
                listeners_per_node: 2,
                garbage_per_invocation: 16,
                dispatches: 3,
            },
            Size::Medium => Self {
                path_depth: 16,
                bystanders: 400,
                listeners_per_node: 4,
                garbage_per_invocation: 256,
                dispatches: 8,
            },
            Size::Large => Self {
                path_depth: 32,
                bystanders: 4_000,
                listeners_per_node: 8,
                garbage_per_invocation: 512,
                dispatches: 16,
            },
        }
    }

    /// Garbage nodes allocated by the dispatch phase.
    #[must_use]
    pub const fn total_garbage(&self) -> u64 {
        self.dispatches as u64
            * self.path_depth as u64
            * self.listeners_per_node as u64
            * self.garbage_per_invocation as u64
    }
}

// ----- Generators -----

/// A document tree. All ids are forgotten when `forget_ids` is set, then a
/// checkpoint is taken.
#[must_use]
pub fn tree(params: &TreeParams, seed: u64) -> Generated {
    let mut rec = Recorder::new(seed);
    let ids = grow_tree(&mut rec, params);
    if params.forget_ids {
        for &id in ids.elements.iter().chain(&ids.texts) {
            rec.forget(id);
        }
    }
    rec.checkpoint();
    rec.finish()
}

/// Listeners on a document, then dispatches with interleaved checkpoints and
/// occasional removals. Finally every id is forgotten and a last checkpoint
/// is taken.
#[must_use]
pub fn listeners(params: &ListenersParams, seed: u64) -> Generated {
    let mut rec = Recorder::new(seed);
    let ids = grow_tree(&mut rec, &params.tree);
    let elements = ids.elements;

    let mut added = 0;
    while added < params.listeners {
        added += rec.random_listener(&elements, params.mix);
    }

    let spacing = (params.dispatches / params.checkpoints.max(1)).max(1);
    for i in 0..params.dispatches {
        if i % spacing == 0 {
            rec.checkpoint();
        }
        if rec.rng.ratio(1, 25) {
            rec.random_listener_removal(&elements);
        }
        let target = *rec.rng.pick(&elements);
        let event = *rec.rng.pick(&EventType::ALL);
        let bubbles = rec.rng.ratio(4, 5);
        rec.dispatch(target, event, bubbles);
    }

    for &id in elements.iter().chain(&ids.texts) {
        rec.forget(id);
    }
    rec.checkpoint();
    rec.finish()
}

/// Random mutation over a bounded pool of live ids.
#[must_use]
pub fn churn(params: &ChurnParams, seed: u64) -> Generated {
    let mut rec = Recorder::new(seed);
    let base = grow_tree(
        &mut rec,
        &TreeParams {
            elements: (params.target_live_ids / 2).max(2),
            max_depth: 8,
            text_per_mille: 200,
            max_attributes: 2,
            forget_ids: false,
        },
    );
    let mut pool: Vec<NodeId> = base.elements.into_iter().chain(base.texts).collect();
    let target_live = params.target_live_ids as usize;

    for step in 1..=params.steps {
        match rec.rng.below(100) {
            0..20 => {
                let tag = *rec.rng.pick(&Tag::CREATABLE);
                let element = rec.element(tag);
                add_attributes(&mut rec, element, 2);
                if rec.rng.ratio(7, 10) {
                    let parent = rec.random_container(&pool);
                    rec.append(parent, element);
                }
                pool.push(element);
            }
            20..24 => {
                let text = rec.text();
                if rec.rng.ratio(7, 10) {
                    let parent = rec.random_container(&pool);
                    rec.append(parent, text);
                }
                pool.push(text);
            }
            24..34 => rec.random_move(&pool),
            34..39 => rec.random_insert_before(&pool),
            39..46 => {
                let node = *rec.rng.pick(&pool);
                rec.remove(node);
            }
            46..55 => {
                if let Some(element) = rec.pick_element(&pool) {
                    let name = *rec.rng.pick(&AttrName::ALL);
                    let value = attribute_value(&mut rec.rng, name);
                    rec.set_attribute(element, name, value);
                }
            }
            55..64 => {
                rec.random_listener(&pool, ListenerMix::CHURN);
            }
            64..67 => rec.random_listener_removal(&pool),
            67..77 => {
                let target = *rec.rng.pick(&pool);
                let event = *rec.rng.pick(&EventType::ALL);
                let bubbles = rec.rng.ratio(4, 5);
                rec.dispatch(target, event, bubbles);
            }
            77..80 => {
                let target = *rec.rng.pick(&pool);
                rec.make_weak(target);
            }
            _ => {
                if pool.len() > target_live / 2 {
                    forget_random(&mut rec, &mut pool);
                }
            }
        }
        while pool.len() > target_live {
            forget_random(&mut rec, &mut pool);
        }
        if step % params.collect_every == 0 {
            rec.collect();
        }
        if step % params.checkpoint_every == 0 {
            rec.checkpoint();
        }
    }

    for id in pool {
        rec.forget(id);
    }
    rec.checkpoint();
    rec.finish()
}

/// GC pressure inside long dispatches. See [`PressureParams`] for the invariant.
#[must_use]
pub fn pressure(params: &PressureParams, seed: u64) -> Generated {
    let mut rec = Recorder::new(seed);
    let html = rec.element(Tag::Html);
    rec.append(NodeId::DOCUMENT, html);
    let body = rec.element(Tag::Body);
    rec.append(html, body);

    let mut chain = Vec::with_capacity(params.path_depth as usize);
    let mut parent = body;
    for _ in 0..params.path_depth {
        let link = rec.element(Tag::Div);
        rec.append(parent, link);
        chain.push(link);
        parent = link;
    }
    for _ in 0..params.bystanders {
        let tag = *rec.rng.pick(&Tag::CREATABLE);
        let bystander = rec.element(tag);
        add_attributes(&mut rec, bystander, 2);
        let host = *rec.rng.pick(&chain);
        rec.append(host, bystander);
    }
    for &link in &chain {
        for _ in 0..params.listeners_per_node {
            rec.add_listener(
                link,
                ListenerSpec {
                    event: EventType::Click,
                    capture: false,
                    behavior: Behavior::AllocGarbage {
                        count: params.garbage_per_invocation,
                    },
                    extra_capture: None,
                },
            );
        }
    }

    rec.checkpoint();
    let target = *chain.last().expect("path_depth > 0");
    for _ in 0..params.dispatches {
        rec.dispatch(target, EventType::Click, true);
    }
    rec.checkpoint();
    rec.finish()
}

// ----- Recording machinery -----

/// Records ops into a [`ScriptBuilder`] and immediately applies each one to the
/// [`Reference`] model, so that generation decisions can query the true
/// current state.
#[derive(Debug)]
struct Recorder {
    builder: ScriptBuilder,
    model: Reference,
    rng: Rng,
}

/// Ids of a generated tree.
#[derive(Debug, Default)]
struct TreeIds {
    elements: Vec<NodeId>,
    texts: Vec<NodeId>,
}

impl Recorder {
    fn new(seed: u64) -> Self {
        Self {
            builder: ScriptBuilder::new(),
            model: Reference::new(),
            rng: Rng::seed_from_u64(seed),
        }
    }

    /// Apply the op the builder just recorded to the model.
    fn commit(&mut self) {
        let op = self.builder.last_op().expect("an op was just recorded");
        if let Err(error) = self.model.apply(op) {
            panic!("generator bug: {op:?} is invalid: {error}");
        }
    }

    fn finish(self) -> Generated {
        let script = self.builder.build();
        let (expected, stats) = self.model.into_parts();
        Generated {
            script,
            expected,
            stats,
        }
    }

    fn element(&mut self, tag: Tag) -> NodeId {
        let id = self.builder.element(tag);
        self.commit();
        id
    }

    fn text(&mut self) -> NodeId {
        let data = format!("t{}", self.rng.below(10_000));
        let id = self.builder.text(data);
        self.commit();
        id
    }

    fn append(&mut self, parent: NodeId, child: NodeId) {
        self.builder.append(parent, child);
        self.commit();
    }

    fn insert_before(&mut self, parent: NodeId, child: NodeId, reference: NodeId) {
        self.builder.insert_before(parent, child, reference);
        self.commit();
    }

    fn remove(&mut self, node: NodeId) {
        self.builder.remove(node);
        self.commit();
    }

    fn set_attribute(&mut self, node: NodeId, name: AttrName, value: String) {
        self.builder.set_attribute(node, name, value);
        self.commit();
    }

    fn add_listener(&mut self, node: NodeId, spec: ListenerSpec) -> ListenerId {
        let id = self.builder.add_listener(node, spec);
        self.commit();
        id
    }

    fn remove_listener(&mut self, node: NodeId, listener: ListenerId) {
        self.builder.remove_listener(node, listener);
        self.commit();
    }

    fn dispatch(&mut self, target: NodeId, event: EventType, bubbles: bool) {
        self.builder.dispatch(target, event, bubbles);
        self.commit();
    }

    fn make_weak(&mut self, target: NodeId) -> WeakId {
        let id = self.builder.make_weak(target);
        self.commit();
        id
    }

    fn forget(&mut self, node: NodeId) {
        self.builder.forget(node);
        self.commit();
    }

    fn collect(&mut self) {
        self.builder.collect();
        self.commit();
    }

    fn checkpoint(&mut self) {
        self.builder.checkpoint();
        self.commit();
    }

    /// A random element from `pool` (by rejection sampling), if one turns up quickly.
    fn pick_element(&mut self, pool: &[NodeId]) -> Option<NodeId> {
        (0..16)
            .map(|_| *self.rng.pick(pool))
            .find(|&id| self.model.is_element(id))
    }

    /// A random node from `pool` that can have children, falling back to the document.
    fn random_container(&mut self, pool: &[NodeId]) -> NodeId {
        (0..16)
            .map(|_| *self.rng.pick(pool))
            .find(|&id| self.model.can_have_children(id))
            .unwrap_or(NodeId::DOCUMENT)
    }

    /// Move a random node under a random container, when that is valid.
    fn random_move(&mut self, pool: &[NodeId]) {
        let child = *self.rng.pick(pool);
        let parent = self.random_container(pool);
        if !self.model.is_inclusive_ancestor(child, parent) {
            self.append(parent, child);
        }
    }

    /// Insert a random node before a random id-bearing child of a random container.
    fn random_insert_before(&mut self, pool: &[NodeId]) {
        let parent = self.random_container(pool);
        let children = self.model.child_ids(parent);
        if children.is_empty() {
            return;
        }
        let reference = *self.rng.pick(&children);
        let child = *self.rng.pick(pool);
        if !self.model.is_inclusive_ancestor(child, parent) {
            self.insert_before(parent, child, reference);
        }
    }

    /// Add one random listener, or a *removal pair*, and return how many
    /// listeners were added.
    ///
    /// A removal pair is an earlier listener that removes the very next one
    /// during dispatch. That is the only way to observe the snapshot's
    /// *removed* flag.
    fn random_listener(&mut self, pool: &[NodeId], mix: ListenerMix) -> u32 {
        let node = if self.rng.ratio(mix.document_per_mille, 1000) {
            NodeId::DOCUMENT
        } else {
            self.pick_element(pool).unwrap_or(NodeId::DOCUMENT)
        };
        let event = *self.rng.pick(&EventType::ALL);
        let capture = self.rng.ratio(1, 3);
        let extra_capture = if self.rng.ratio(mix.extra_capture_per_mille, 1000) {
            Some(*self.rng.pick(pool))
        } else {
            None
        };

        if self.rng.ratio(mix.removal_pair_per_mille, 1000) {
            let victim = ListenerId(self.builder.peek_next_listener().0 + 1);
            let remover = Behavior::RemoveListener { listener: victim };
            self.add_listener(
                node,
                ListenerSpec {
                    event,
                    capture,
                    behavior: remover,
                    extra_capture,
                },
            );
            let added = self.add_listener(
                node,
                ListenerSpec {
                    event,
                    capture,
                    behavior: Behavior::LogOnly,
                    extra_capture: None,
                },
            );
            debug_assert_eq!(added, victim);
            return 2;
        }

        let behavior = self.random_behavior(node);
        self.add_listener(
            node,
            ListenerSpec {
                event,
                capture,
                behavior,
                extra_capture,
            },
        );
        1
    }

    fn random_behavior(&mut self, node: NodeId) -> Behavior {
        let element = self.model.is_element(node);
        match self.rng.below(100) {
            0..15 if element => {
                let name = *self.rng.pick(&AttrName::ALL);
                Behavior::SetAttribute {
                    name,
                    value: attribute_value(&mut self.rng, name).into(),
                }
            }
            15..22 => Behavior::StopPropagation,
            22..30 if element => Behavior::AppendFresh {
                tag: Tag::Span,
                count: small_u16(self.rng.range(1..4)),
            },
            30..38 => Behavior::AllocGarbage {
                count: small_u16(self.rng.range(1..9)),
            },
            38..44 => {
                let on_node = self.model.listeners_on(node);
                if on_node.is_empty() {
                    Behavior::LogOnly
                } else {
                    Behavior::RemoveListener {
                        listener: *self.rng.pick(&on_node),
                    }
                }
            }
            44..47 if element => Behavior::RemoveSelf,
            _ => Behavior::LogOnly,
        }
    }

    /// Remove a random listener from a random node in `pool` (driver-initiated).
    fn random_listener_removal(&mut self, pool: &[NodeId]) {
        let node = *self.rng.pick(pool);
        let on_node = self.model.listeners_on(node);
        if !on_node.is_empty() {
            let listener = *self.rng.pick(&on_node);
            self.remove_listener(node, listener);
        }
    }
}

/// Breadth-first growth: every queue entry owns a budget of descendants still
/// to be created. Most nodes get 1–4 children; one in eight is list-like,
/// with 8–32. At `max_depth`, the remaining budget becomes leaves.
fn grow_tree(rec: &mut Recorder, params: &TreeParams) -> TreeIds {
    let mut ids = TreeIds::default();
    let html = rec.element(Tag::Html);
    rec.append(NodeId::DOCUMENT, html);
    let body = rec.element(Tag::Body);
    rec.append(html, body);
    ids.elements.extend([html, body]);

    let mut queue = VecDeque::from([(body, Tag::Body, 2_u32, params.elements.saturating_sub(2))]);
    while let Some((parent, parent_tag, depth, budget)) = queue.pop_front() {
        if budget == 0 {
            continue;
        }
        let fanout = if depth >= params.max_depth {
            budget
        } else if rec.rng.ratio(1, 8) {
            rec.rng.range(8..33)
        } else {
            rec.rng.range(1..5)
        }
        .min(budget);

        let mut rest = budget - fanout;
        for i in 0..fanout {
            let siblings_left = fanout - i;
            let share = if siblings_left == 1 {
                rest
            } else {
                rec.rng.below(2 * (rest / siblings_left) + 1).min(rest)
            };
            rest -= share;

            let tag = child_tag(&mut rec.rng, parent_tag);
            let child = rec.element(tag);
            add_attributes(rec, child, params.max_attributes);
            rec.append(parent, child);
            if rec.rng.ratio(params.text_per_mille, 1000) {
                let text = rec.text();
                rec.append(child, text);
                ids.texts.push(text);
            }
            ids.elements.push(child);
            queue.push_back((child, tag, depth + 1, share));
        }
    }
    ids
}

fn child_tag(rng: &mut Rng, parent: Tag) -> Tag {
    const WEIGHTED: [Tag; 11] = [
        Tag::Div,
        Tag::Div,
        Tag::Div,
        Tag::Span,
        Tag::Span,
        Tag::P,
        Tag::P,
        Tag::A,
        Tag::Ul,
        Tag::Section,
        Tag::Button,
    ];
    if parent == Tag::Ul {
        Tag::Li
    } else {
        *rng.pick(&WEIGHTED)
    }
}

/// Give `node` up to `max` distinct attributes, chosen by a partial
/// Fisher–Yates shuffle over the names.
fn add_attributes(rec: &mut Recorder, node: NodeId, max: u32) {
    let mut names = AttrName::ALL;
    let count = rec.rng.below(max.min(8) + 1) as usize;
    for i in 0..count {
        let remaining = u32::try_from(names.len() - i).expect("8 names");
        let j = i + rec.rng.below(remaining) as usize;
        names.swap(i, j);
        let value = attribute_value(&mut rec.rng, names[i]);
        rec.set_attribute(node, names[i], value);
    }
}

fn attribute_value(rng: &mut Rng, name: AttrName) -> String {
    match name {
        AttrName::Class => format!("c{} c{}", rng.below(64), rng.below(64)),
        AttrName::Id => format!("n{}", rng.below(1_000_000)),
        AttrName::Href => format!("/p/{}", rng.below(4_096)),
        _ => format!("v{}", rng.below(1_000)),
    }
}

fn forget_random(rec: &mut Recorder, pool: &mut Vec<NodeId>) {
    let len = u32::try_from(pool.len()).expect("pool fits in u32");
    let id = pool.swap_remove(rec.rng.below(len) as usize);
    rec.forget(id);
}

fn small_u16(x: u32) -> u16 {
    u16::try_from(x).expect("small count")
}

#[cfg(test)]
mod tests;
