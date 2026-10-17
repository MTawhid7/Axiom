//! Write barriers (ADR-0008 SH9, C3), for S3-Q5.
//!
//! Every write of a handle-valued field goes through a setter that the
//! [`setters!`](crate::setters) macro generates, and every setter calls
//! [`Barrier::post_write`]. The stop-the-world collector needs no barrier, so
//! the default, [`NoBarrier`], compiles to nothing. [`CardBarrier`] is what a
//! generational collector would add: it marks the card (a group of objects)
//! that the written object belongs to, so that a minor collection scans only
//! dirty cards for references into the nursery.

use core::fmt;

use crate::value::MAX_ARENAS;

/// What runs when the heap allocates, when a handle-valued field is written,
/// and after a collection.
pub trait Barrier: Default + fmt::Debug {
    fn on_alloc(&mut self, arena: usize, index: usize);
    fn post_write(&mut self, arena: usize, index: usize);
    fn after_collection(&mut self, arena_lens: &[usize]);
}

/// No barrier: the stop-the-world collector's choice.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoBarrier;

impl Barrier for NoBarrier {
    #[inline(always)]
    fn on_alloc(&mut self, _: usize, _: usize) {}

    #[inline(always)]
    fn post_write(&mut self, _: usize, _: usize) {}

    #[inline(always)]
    fn after_collection(&mut self, _: &[usize]) {}
}

/// Objects per card: 512, so a card table costs one byte per 512 objects.
pub const CARD_SHIFT: u32 = 9;
const CLEAN: u8 = 0;
const DIRTY: u8 = 1;

/// Card marking: one byte per [`1 << CARD_SHIFT`](CARD_SHIFT) objects of each
/// arena. A write marks the written object's card. This collector never
/// reads the cards (it has no nursery); they are cleared after each
/// collection, as a minor collection would clear them.
#[derive(Debug, Default)]
pub struct CardBarrier {
    cards: [Vec<u8>; MAX_ARENAS],
}

impl CardBarrier {
    /// The number of dirty cards, across all arenas.
    #[must_use]
    pub fn dirty_cards(&self) -> usize {
        self.cards
            .iter()
            .flatten()
            .filter(|&&card| card == DIRTY)
            .count()
    }
}

impl Barrier for CardBarrier {
    #[inline]
    fn on_alloc(&mut self, arena: usize, index: usize) {
        let card = index >> CARD_SHIFT;
        let cards = &mut self.cards[arena];
        if cards.len() <= card {
            cards.resize(card + 1, CLEAN);
        }
    }

    #[inline]
    fn post_write(&mut self, arena: usize, index: usize) {
        self.cards[arena][index >> CARD_SHIFT] = DIRTY;
    }

    fn after_collection(&mut self, arena_lens: &[usize]) {
        for (cards, &len) in self.cards.iter_mut().zip(arena_lens) {
            cards.clear();
            cards.resize(len.div_ceil(1 << CARD_SHIFT), CLEAN);
        }
    }
}
