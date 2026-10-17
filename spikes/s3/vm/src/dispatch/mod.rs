//! How the interpreter finds and runs the next instruction (S3-Q3). Four
//! techniques share every operation's semantics ([`crate::machine`]) and
//! differ only here:
//!
//! | Technique | Next instruction | Where `pc` and the window live |
//! |---|---|---|
//! | [`Match`](Technique::Match) | one `match` on the opcode: a jump table with one shared indirect branch | locals, so registers |
//! | [`Table`](Technique::Table) | an indirect call through a 256-entry table of handlers, which return to the loop | a state struct in memory |
//! | [`Threaded`](Technique::Threaded) | each handler returns the next instruction's handler, stored in the decoded instruction; a trampoline calls it | a state struct in memory |
//! | `Tail` (nightly, feature `nightly-tail`) | each handler tail-calls the next one with `become`: one indirect branch per handler | arguments, so registers |

pub(crate) mod matched;
pub(crate) mod table;
#[cfg(feature = "nightly-tail")]
pub(crate) mod tail;
pub(crate) mod threaded;

use core::fmt;

/// A dispatch technique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Technique {
    Match,
    Table,
    Threaded,
    #[cfg(feature = "nightly-tail")]
    Tail,
}

impl Technique {
    /// Every technique this build has.
    pub const ALL: &[Self] = &[
        Self::Match,
        Self::Table,
        Self::Threaded,
        #[cfg(feature = "nightly-tail")]
        Self::Tail,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Table => "table",
            Self::Threaded => "threaded",
            #[cfg(feature = "nightly-tail")]
            Self::Tail => "tail",
        }
    }

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|t| t.name() == name)
    }
}

impl fmt::Display for Technique {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
