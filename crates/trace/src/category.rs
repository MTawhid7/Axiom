//! Categories and levels (OB6), and the process-wide mask that switches the
//! detailed tier on per category (OB7).

use std::sync::atomic::{AtomicU64, Ordering};

/// A record's category: one per subsystem and concern (OB6). The collector,
/// the flight recorder and the exports treat categories generically, so
/// adding one is a variant here and its call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Category {
    /// Tasks on every thread loop; PF19's task ages.
    Task = 0,
    /// Messages, flows and violations on IPC channels.
    Ipc = 1,
    /// Navigations.
    Nav = 2,
    /// Loads and the loader's streams.
    Load = 3,
    /// The HTML and XML parsers.
    Parse = 4,
    /// Style: matching, the cascade, computed values.
    Style = 5,
    /// Layout.
    Layout = 6,
    /// Painting into display lists.
    Paint = 7,
    /// The compositor and the presenter.
    Composite = 8,
    /// The network process; MA1's network log.
    Net = 9,
    /// The collector of Pattern B's heap.
    Gc = 10,
    /// Fonts, shaping and line breaking.
    Text = 11,
    /// Image decoding.
    Image = 12,
    /// Process health: footprints, budgets, drops and panics.
    Health = 13,
}

impl Category {
    /// Every category, in ID order.
    pub const ALL: [Self; 14] = [
        Self::Task,
        Self::Ipc,
        Self::Nav,
        Self::Load,
        Self::Parse,
        Self::Style,
        Self::Layout,
        Self::Paint,
        Self::Composite,
        Self::Net,
        Self::Gc,
        Self::Text,
        Self::Image,
        Self::Health,
    ];

    /// The category's ID in records.
    #[must_use]
    pub const fn id(self) -> u8 {
        self as u8
    }

    /// The category with this ID, if there is one.
    #[must_use]
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Task),
            1 => Some(Self::Ipc),
            2 => Some(Self::Nav),
            3 => Some(Self::Load),
            4 => Some(Self::Parse),
            5 => Some(Self::Style),
            6 => Some(Self::Layout),
            7 => Some(Self::Paint),
            8 => Some(Self::Composite),
            9 => Some(Self::Net),
            10 => Some(Self::Gc),
            11 => Some(Self::Text),
            12 => Some(Self::Image),
            13 => Some(Self::Health),
            _ => None,
        }
    }

    /// The category's name, as exports show it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Ipc => "ipc",
            Self::Nav => "nav",
            Self::Load => "load",
            Self::Parse => "parse",
            Self::Style => "style",
            Self::Layout => "layout",
            Self::Paint => "paint",
            Self::Composite => "composite",
            Self::Net => "net",
            Self::Gc => "gc",
            Self::Text => "text",
            Self::Image => "image",
            Self::Health => "health",
        }
    }

    const fn bit(self) -> u64 {
        1_u64.wrapping_shl(self as u32)
    }
}

/// A log's severity (OB6), most severe first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Level {
    /// A failure the owner must see: a violation, a crash, a budget breached.
    Error = 0,
    /// A degraded mode or a recovery.
    Warn = 1,
    /// A notable event, kept in the always-on tier.
    Info = 2,
    /// Detail for development, compiled out of release builds.
    Debug = 3,
}

impl Level {
    /// The level with this ID, if there is one.
    #[must_use]
    pub const fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Error),
            1 => Some(Self::Warn),
            2 => Some(Self::Info),
            3 => Some(Self::Debug),
            _ => None,
        }
    }

    /// The level's name, as exports show it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

/// The most detailed level this build records (OB6): `Debug` in development
/// builds, `Info` in release builds, where a `Debug` log costs nothing.
pub const MAX_LEVEL: Level = if cfg!(debug_assertions) {
    Level::Debug
} else {
    Level::Info
};

/// The detailed tier's mask: bit `n` set means category `n` records detail.
static DETAILED: AtomicU64 = AtomicU64::new(0);

/// Switches `category`'s detailed tier on, in this process.
pub fn enable(category: Category) {
    DETAILED.fetch_or(category.bit(), Ordering::Relaxed);
}

/// Switches `category`'s detailed tier off, in this process.
pub fn disable(category: Category) {
    DETAILED.fetch_and(!category.bit(), Ordering::Relaxed);
}

/// Whether `category` records detail: a call site of the detailed tier checks
/// this first, and a disabled category costs this one relaxed load (OB9).
#[inline]
#[must_use]
pub fn enabled(category: Category) -> bool {
    DETAILED.load(Ordering::Relaxed) & category.bit() != 0
}

#[cfg(test)]
mod tests {
    use super::{Category, Level, disable, enable, enabled};

    #[test]
    fn ids_round_trip_and_unknown_ids_are_refused() {
        for (index, category) in Category::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(category.id()), index);
            assert_eq!(Category::from_id(category.id()), Some(category));
        }
        assert_eq!(Category::from_id(14), None);
        assert_eq!(Level::from_id(Level::Debug as u8), Some(Level::Debug));
        assert_eq!(Level::from_id(4), None);
    }

    #[test]
    fn the_mask_switches_one_category_at_a_time() {
        // This test owns the Gc and Image categories; others use their own.
        enable(Category::Gc);
        assert!(enabled(Category::Gc));
        assert!(!enabled(Category::Image));
        disable(Category::Gc);
        assert!(!enabled(Category::Gc));
    }
}
