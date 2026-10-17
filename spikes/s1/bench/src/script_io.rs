//! A line-oriented text format for scripts.
//!
//! It lets a measurement process run a script without generating it. The
//! generator's own memory (the reference model, the expected outcome) would
//! otherwise be part of the process's peak RSS, and would hide the heap's.
//!
//! ```text
//! s1-script 1
//! slots <node slots> <listener slots> <weak slots>
//! expect <outcome digest, 16 hex digits>
//! <one op per line>
//! ```
//!
//! | Op | Line |
//! |---|---|
//! | `CreateElement` | `E <id> <tag>` |
//! | `CreateText` | `T <id> <data…>` |
//! | `AppendChild` | `A <parent> <child>` |
//! | `InsertBefore` | `I <parent> <child> <reference>` |
//! | `Remove` | `R <node>` |
//! | `SetAttribute` | `S <node> <name> <value…>` |
//! | `AddListener` | `L <node> <listener> <event> <capture 0/1> <extra id or -> <behaviour…>` |
//! | `RemoveListener` | `X <node> <listener>` |
//! | `Dispatch` | `D <target> <event> <bubbles 0/1>` |
//! | `MakeWeak` | `W <weak> <target>` |
//! | `Forget` | `F <node>` |
//! | `Collect` | `C` |
//! | `Checkpoint` | `K` |
//!
//! Behaviours: `log`, `stop`, `remove-self`, `garbage <count>`,
//! `append <tag> <count>`, `remove-listener <id>`, `set <name> <value…>`.
//! A `…` field runs to the end of the line, so it may contain spaces, but not
//! newlines.

use core::fmt;
use std::io::{self, BufRead, Write};

use s1_scenarios::{
    AttrName, Behavior, EventType, ListenerId, ListenerSpec, NodeId, Op, Outcome, Script, Tag,
    WeakId,
};

const MAGIC: &str = "s1-script 1";

const TAGS: [Tag; 11] = [
    Tag::Document,
    Tag::Html,
    Tag::Body,
    Tag::Div,
    Tag::Span,
    Tag::P,
    Tag::A,
    Tag::Ul,
    Tag::Li,
    Tag::Section,
    Tag::Button,
];

const fn event_name(event: EventType) -> &'static str {
    match event {
        EventType::Click => "click",
        EventType::Input => "input",
        EventType::Focus => "focus",
        EventType::Custom => "custom",
    }
}

/// Write `script` with the digest of the outcome it must produce.
///
/// # Panics
/// If a string field contains a newline (generated scripts never do).
pub fn write(out: &mut impl Write, script: &Script, expected: &Outcome) -> io::Result<()> {
    writeln!(out, "{MAGIC}")?;
    writeln!(
        out,
        "slots {} {} {}",
        script.node_slots, script.listener_slots, script.weak_slots
    )?;
    writeln!(out, "expect {:016x}", expected.digest())?;
    for op in &script.ops {
        write_op(out, op)?;
    }
    Ok(())
}

fn single_line(s: &str) -> &str {
    assert!(
        !s.contains('\n'),
        "script strings must not contain newlines"
    );
    s
}

fn write_op(out: &mut impl Write, op: &Op) -> io::Result<()> {
    match op {
        Op::CreateElement { id, tag } => writeln!(out, "E {} {}", id.0, tag.as_str()),
        Op::CreateText { id, data } => writeln!(out, "T {} {}", id.0, single_line(data)),
        Op::AppendChild { parent, child } => writeln!(out, "A {} {}", parent.0, child.0),
        Op::InsertBefore {
            parent,
            child,
            reference,
        } => writeln!(out, "I {} {} {}", parent.0, child.0, reference.0),
        Op::Remove { node } => writeln!(out, "R {}", node.0),
        Op::SetAttribute { node, name, value } => {
            writeln!(out, "S {} {} {}", node.0, name.as_str(), single_line(value))
        }
        Op::AddListener {
            node,
            listener,
            spec,
        } => {
            write!(
                out,
                "L {} {} {} {} ",
                node.0,
                listener.0,
                event_name(spec.event),
                u8::from(spec.capture)
            )?;
            match spec.extra_capture {
                Some(extra) => write!(out, "{} ", extra.0)?,
                None => write!(out, "- ")?,
            }
            match &spec.behavior {
                Behavior::LogOnly => writeln!(out, "log"),
                Behavior::StopPropagation => writeln!(out, "stop"),
                Behavior::RemoveSelf => writeln!(out, "remove-self"),
                Behavior::AllocGarbage { count } => writeln!(out, "garbage {count}"),
                Behavior::AppendFresh { tag, count } => {
                    writeln!(out, "append {} {count}", tag.as_str())
                }
                Behavior::RemoveListener { listener } => {
                    writeln!(out, "remove-listener {}", listener.0)
                }
                Behavior::SetAttribute { name, value } => {
                    writeln!(out, "set {} {}", name.as_str(), single_line(value))
                }
            }
        }
        Op::RemoveListener { node, listener } => writeln!(out, "X {} {}", node.0, listener.0),
        Op::Dispatch {
            target,
            event,
            bubbles,
        } => writeln!(
            out,
            "D {} {} {}",
            target.0,
            event_name(*event),
            u8::from(*bubbles)
        ),
        Op::MakeWeak { weak, target } => writeln!(out, "W {} {}", weak.0, target.0),
        Op::Forget { node } => writeln!(out, "F {}", node.0),
        Op::Collect => writeln!(out, "C"),
        Op::Checkpoint => writeln!(out, "K"),
    }
}

/// A parse failure, with its 1-based line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// A parsed script file: the script, and the digest of its expected outcome.
#[derive(Clone, Debug)]
pub struct ScriptFile {
    pub script: Script,
    pub expected_digest: u64,
}

/// Read a script written by [`write`].
pub fn read(input: impl BufRead) -> Result<ScriptFile, ParseError> {
    let mut lines = input.lines().enumerate();
    let mut next_line = |what: &str| -> Result<(usize, String), ParseError> {
        match lines.next() {
            Some((i, Ok(line))) => Ok((i + 1, line)),
            Some((i, Err(e))) => Err(ParseError {
                line: i + 1,
                message: e.to_string(),
            }),
            None => Err(ParseError {
                line: 0,
                message: format!("missing {what}"),
            }),
        }
    };

    let (n, magic) = next_line("header")?;
    if magic != MAGIC {
        return Err(error(n, format!("expected {MAGIC:?}")));
    }
    let (n, slots) = next_line("slots")?;
    let mut f = Fields::new(n, &slots);
    f.keyword("slots")?;
    let (node_slots, listener_slots, weak_slots) = (f.int()?, f.int()?, f.int()?);
    let (n, expect) = next_line("expect")?;
    let mut f = Fields::new(n, &expect);
    f.keyword("expect")?;
    let digest = f.word()?;
    let expected_digest = u64::from_str_radix(digest, 16)
        .map_err(|e| error(n, format!("bad digest {digest:?}: {e}")))?;

    let mut ops = Vec::new();
    for (i, line) in lines {
        let n = i + 1;
        let line = line.map_err(|e| error(n, e.to_string()))?;
        ops.push(parse_op(&mut Fields::new(n, &line))?);
    }
    Ok(ScriptFile {
        script: Script {
            ops,
            node_slots,
            listener_slots,
            weak_slots,
        },
        expected_digest,
    })
}

fn error(line: usize, message: String) -> ParseError {
    ParseError { line, message }
}

/// Whitespace-separated fields of one line, read left to right.
struct Fields<'a> {
    line: usize,
    rest: &'a str,
}

impl<'a> Fields<'a> {
    const fn new(line: usize, text: &'a str) -> Self {
        Self { line, rest: text }
    }

    fn word(&mut self) -> Result<&'a str, ParseError> {
        let (word, rest) = self.rest.split_once(' ').unwrap_or((self.rest, ""));
        if word.is_empty() {
            return Err(error(self.line, "unexpected end of line".to_owned()));
        }
        self.rest = rest;
        Ok(word)
    }

    /// Everything left on the line.
    fn rest(&mut self) -> &'a str {
        core::mem::take(&mut self.rest)
    }

    fn keyword(&mut self, expected: &str) -> Result<(), ParseError> {
        let word = self.word()?;
        if word == expected {
            Ok(())
        } else {
            Err(error(
                self.line,
                format!("expected {expected:?}, got {word:?}"),
            ))
        }
    }

    fn int<T: core::str::FromStr>(&mut self) -> Result<T, ParseError>
    where
        T::Err: fmt::Display,
    {
        let word = self.word()?;
        word.parse()
            .map_err(|e| error(self.line, format!("bad number {word:?}: {e}")))
    }

    fn node(&mut self) -> Result<NodeId, ParseError> {
        self.int().map(NodeId)
    }

    fn flag(&mut self) -> Result<bool, ParseError> {
        match self.word()? {
            "0" => Ok(false),
            "1" => Ok(true),
            other => Err(error(self.line, format!("bad flag {other:?}"))),
        }
    }

    fn tag(&mut self) -> Result<Tag, ParseError> {
        let word = self.word()?;
        TAGS.into_iter()
            .find(|t| t.as_str() == word)
            .ok_or_else(|| error(self.line, format!("unknown tag {word:?}")))
    }

    fn attr(&mut self) -> Result<AttrName, ParseError> {
        let word = self.word()?;
        AttrName::ALL
            .into_iter()
            .find(|a| a.as_str() == word)
            .ok_or_else(|| error(self.line, format!("unknown attribute {word:?}")))
    }

    fn event(&mut self) -> Result<EventType, ParseError> {
        let word = self.word()?;
        EventType::ALL
            .into_iter()
            .find(|&e| event_name(e) == word)
            .ok_or_else(|| error(self.line, format!("unknown event {word:?}")))
    }

    fn end(&self) -> Result<(), ParseError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(error(self.line, format!("trailing fields {:?}", self.rest)))
        }
    }
}

fn parse_op(f: &mut Fields<'_>) -> Result<Op, ParseError> {
    let op = match f.word()? {
        "E" => Op::CreateElement {
            id: f.node()?,
            tag: f.tag()?,
        },
        "T" => Op::CreateText {
            id: f.node()?,
            data: f.rest().into(),
        },
        "A" => Op::AppendChild {
            parent: f.node()?,
            child: f.node()?,
        },
        "I" => Op::InsertBefore {
            parent: f.node()?,
            child: f.node()?,
            reference: f.node()?,
        },
        "R" => Op::Remove { node: f.node()? },
        "S" => Op::SetAttribute {
            node: f.node()?,
            name: f.attr()?,
            value: f.rest().into(),
        },
        "L" => {
            let node = f.node()?;
            let listener = ListenerId(f.int()?);
            let event = f.event()?;
            let capture = f.flag()?;
            let extra_capture = match f.word()? {
                "-" => None,
                id => Some(NodeId(id.parse().map_err(|e| {
                    error(f.line, format!("bad extra capture {id:?}: {e}"))
                })?)),
            };
            let behavior = parse_behavior(f)?;
            Op::AddListener {
                node,
                listener,
                spec: ListenerSpec {
                    event,
                    capture,
                    behavior,
                    extra_capture,
                },
            }
        }
        "X" => Op::RemoveListener {
            node: f.node()?,
            listener: ListenerId(f.int()?),
        },
        "D" => Op::Dispatch {
            target: f.node()?,
            event: f.event()?,
            bubbles: f.flag()?,
        },
        "W" => Op::MakeWeak {
            weak: WeakId(f.int()?),
            target: f.node()?,
        },
        "F" => Op::Forget { node: f.node()? },
        "C" => Op::Collect,
        "K" => Op::Checkpoint,
        other => return Err(error(f.line, format!("unknown op {other:?}"))),
    };
    f.end()?;
    Ok(op)
}

fn parse_behavior(f: &mut Fields<'_>) -> Result<Behavior, ParseError> {
    Ok(match f.word()? {
        "log" => Behavior::LogOnly,
        "stop" => Behavior::StopPropagation,
        "remove-self" => Behavior::RemoveSelf,
        "garbage" => Behavior::AllocGarbage { count: f.int()? },
        "append" => Behavior::AppendFresh {
            tag: f.tag()?,
            count: f.int()?,
        },
        "remove-listener" => Behavior::RemoveListener {
            listener: ListenerId(f.int()?),
        },
        "set" => Behavior::SetAttribute {
            name: f.attr()?,
            value: f.rest().into(),
        },
        other => return Err(error(f.line, format!("unknown behaviour {other:?}"))),
    })
}

#[cfg(test)]
mod tests {
    use s1_scenarios::{Size, Workload, edge_cases};

    use super::*;

    fn round_trip(script: &Script, expected: &Outcome) {
        let mut bytes = Vec::new();
        write(&mut bytes, script, expected).expect("writes to memory");
        let file = read(bytes.as_slice()).expect("parses its own output");
        assert_eq!(file.script.ops, script.ops);
        assert_eq!(
            (
                file.script.node_slots,
                file.script.listener_slots,
                file.script.weak_slots
            ),
            (script.node_slots, script.listener_slots, script.weak_slots)
        );
        assert_eq!(file.expected_digest, expected.digest());
    }

    #[test]
    fn generated_workloads_round_trip() {
        for workload in Workload::ALL {
            for size in [Size::Small, Size::Medium] {
                let generated = workload.generate(size, 3);
                round_trip(&generated.script, &generated.expected);
            }
        }
    }

    #[test]
    fn edge_cases_round_trip() {
        for (name, script) in edge_cases::all() {
            let (expected, _) = s1_scenarios::reference::run(&script)
                .unwrap_or_else(|e| panic!("{name} is valid: {e}"));
            round_trip(&script, &expected);
        }
    }

    #[test]
    fn errors_name_the_line() {
        let text = format!("{MAGIC}\nslots 2 0 0\nexpect 0000000000000000\nE 1 div\nQ 7\n");
        let err = read(text.as_bytes()).expect_err("Q is not an op");
        assert_eq!(err.line, 5);
        let text = format!("{MAGIC}\nslots 2 0 0\nexpect 0000000000000000\nE 1 div extra\n");
        assert_eq!(read(text.as_bytes()).expect_err("trailing").line, 4);
    }
}
