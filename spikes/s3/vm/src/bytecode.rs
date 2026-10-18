//! The bytecode: a register machine with 28 operations, and its verifier.
//!
//! # The machine
//!
//! - **Registers.** A frame has 256 registers, `R[0]`–`R[255]`, holding
//!   [`Value`]s. Parameters arrive in `R[0]`, `R[1]`, … A missing argument is
//!   `undefined`. The running closure sits just below `R[0]`, so `GET_ENV` and
//!   `SET_ENV` can reach its environment.
//! - **Calls** are Lua's: `CALL a, b` calls `R[a]` with the `b` arguments in
//!   `R[a+1]`–`R[a+b]` and puts the result in `R[a]`. The callee's frame
//!   starts at the caller's `R[a+1]`, so arguments are never copied, and
//!   **every register above `R[a]` is clobbered by a call**. The reference
//!   evaluator poisons them, so a program that reads one fails there.
//! - **Numbers** are JavaScript's doubles. 32-bit integers are a faster
//!   representation of the same values, never different semantics: `-0`, `NaN`
//!   and overflow behave as with doubles.
//! - **Objects** have four slots. Converting an object to a number calls the
//!   closure in its slot 3, if there is one, with the object as argument: the
//!   `valueOf`-style coercion hook. It must return a primitive.
//! - **Arrays** are dense vectors. Reading past the end gives `undefined`;
//!   writing past the end fills the gap with `undefined`.
//!
//! This is JavaScript-shaped, not JavaScript: there are no strings, prototypes
//! or exceptions, and operators are numeric only. That is enough to exercise
//! what spike S3 measures: dispatch, allocation, rooting, and builtins that
//! call back into bytecode.
//!
//! # Encoding
//!
//! An instruction is 4 bytes: an opcode and three 8-bit operands `a`, `b` and
//! `c`. `bx` reads `b` and `c` as one 16-bit operand, `sbx` as a signed one,
//! and `sc` reads `c` as signed. Jump offsets are relative to the next
//! instruction.

use core::fmt;

use s3_gc::Value;

/// Opcodes, with their operands and meaning.
pub mod op {
    /// `a, sbx`: `R[a] = sbx`
    pub const LOAD_INT: u8 = 0;
    /// `a, bx`: `R[a] = K[bx]`
    pub const LOAD_CONST: u8 = 1;
    /// `a, b`: `R[a] = [undefined, null, false, true][b]`
    pub const LOAD_MISC: u8 = 2;
    /// `a, b`: `R[a] = R[b]`
    pub const MOVE: u8 = 3;
    /// `a, b, c`: `R[a] = R[b] + R[c]`
    pub const ADD: u8 = 4;
    /// `a, b, c`: `R[a] = R[b] - R[c]`
    pub const SUB: u8 = 5;
    /// `a, b, c`: `R[a] = R[b] * R[c]`
    pub const MUL: u8 = 6;
    /// `a, b, c`: `R[a] = R[b] / R[c]`
    pub const DIV: u8 = 7;
    /// `a, b, c`: `R[a] = R[b] % R[c]`
    pub const MOD: u8 = 8;
    /// `a, b, sc`: `R[a] = R[b] + sc`
    pub const ADD_IMM: u8 = 9;
    /// `a, b, c`: `R[a] = R[b] < R[c]`
    pub const LT: u8 = 10;
    /// `a, b, c`: `R[a] = R[b] <= R[c]`
    pub const LE: u8 = 11;
    /// `a, b, c`: `R[a] = R[b] === R[c]`
    pub const EQ: u8 = 12;
    /// `sbx`: `pc += sbx`
    pub const JUMP: u8 = 13;
    /// `a, sbx`: `if R[a] { pc += sbx }`
    pub const JUMP_IF: u8 = 14;
    /// `a, sbx`: `if !R[a] { pc += sbx }`
    pub const JUMP_IF_NOT: u8 = 15;
    /// `a`: `R[a] = {}`, four slots of `undefined`
    pub const NEW_OBJECT: u8 = 16;
    /// `a, b, c`: `R[a] = R[b].slot[c]`
    pub const GET_SLOT: u8 = 17;
    /// `a, b, c`: `R[a].slot[b] = R[c]`
    pub const SET_SLOT: u8 = 18;
    /// `a, b`: `R[a] = new Array(R[b])`, filled with `undefined`
    pub const NEW_ARRAY: u8 = 19;
    /// `a, b, c`: `R[a] = R[b][R[c]]`
    pub const GET_INDEX: u8 = 20;
    /// `a, b, c`: `R[a][R[b]] = R[c]`
    pub const SET_INDEX: u8 = 21;
    /// `a, b`: `R[a] = R[b].length`
    pub const LENGTH: u8 = 22;
    /// `a, b, c`: `R[a] = ` a closure of function `b` over the environment `R[c]`
    pub const CLOSURE: u8 = 23;
    /// `a, b`: `R[a] = env.slot[b]`
    pub const GET_ENV: u8 = 24;
    /// `a, b`: `env.slot[a] = R[b]`
    pub const SET_ENV: u8 = 25;
    /// `a, b`: `R[a] = R[a](R[a+1], …, R[a+b])`
    pub const CALL: u8 = 26;
    /// `a`: return `R[a]`
    pub const RETURN: u8 = 27;

    /// The number of opcodes.
    pub const COUNT: usize = 28;

    /// Opcode names, for disassembly and per-opcode counts.
    pub const NAMES: [&str; COUNT] = [
        "LOAD_INT",
        "LOAD_CONST",
        "LOAD_MISC",
        "MOVE",
        "ADD",
        "SUB",
        "MUL",
        "DIV",
        "MOD",
        "ADD_IMM",
        "LT",
        "LE",
        "EQ",
        "JUMP",
        "JUMP_IF",
        "JUMP_IF_NOT",
        "NEW_OBJECT",
        "GET_SLOT",
        "SET_SLOT",
        "NEW_ARRAY",
        "GET_INDEX",
        "SET_INDEX",
        "LENGTH",
        "CLOSURE",
        "GET_ENV",
        "SET_ENV",
        "CALL",
        "RETURN",
    ];
}

/// The values `LOAD_MISC` loads, by operand `b`.
pub const MISC: [Value; 4] = [Value::UNDEFINED, Value::NULL, Value::FALSE, Value::TRUE];

/// One instruction.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Instr {
    pub op: u8,
    pub a: u8,
    pub b: u8,
    pub c: u8,
}

impl Instr {
    #[must_use]
    pub const fn new(op: u8, a: u8, b: u8, c: u8) -> Self {
        Self { op, a, b, c }
    }

    /// An instruction with a 16-bit operand in `b` and `c`.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "splitting a u16 into its bytes"
    )]
    pub const fn with_bx(op: u8, a: u8, bx: u16) -> Self {
        Self::new(op, a, bx as u8, (bx >> 8) as u8)
    }

    /// `b` and `c` as one unsigned 16-bit operand.
    #[inline]
    #[must_use]
    pub const fn bx(self) -> u16 {
        self.b as u16 | (self.c as u16) << 8
    }

    /// `b` and `c` as one signed 16-bit operand.
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_wrap, reason = "reinterpreting the operand")]
    pub const fn sbx(self) -> i16 {
        self.bx() as i16
    }

    /// `c` as a signed 8-bit operand.
    #[inline]
    #[must_use]
    #[expect(clippy::cast_possible_wrap, reason = "reinterpreting the operand")]
    pub const fn sc(self) -> i8 {
        self.c as i8
    }
}

impl fmt::Debug for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = op::NAMES.get(usize::from(self.op)).unwrap_or(&"?");
        write!(f, "{name} {} {} {}", self.a, self.b, self.c)
    }
}

/// A function: code, constants, and how many parameters it takes.
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub params: u8,
    pub code: Vec<Instr>,
    /// Constants for `LOAD_CONST`: numbers, `undefined`, `null`, booleans and
    /// builtins only, since the collector never traces them.
    pub consts: Vec<Value>,
}

/// A program. Function 0 is the entry point.
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub functions: Vec<Function>,
}

impl Program {
    /// Check that the program can't index out of bounds or fall off the end
    /// of a function, so interpreters may rely on it.
    pub fn verify(&self) -> Result<(), String> {
        if self.functions.is_empty() {
            return Err("a program needs an entry function".into());
        }
        for (index, function) in self.functions.iter().enumerate() {
            verify_function(self, function)
                .map_err(|e| format!("function {index} ({}): {e}", function.name))?;
        }
        Ok(())
    }
}

fn verify_function(program: &Program, f: &Function) -> Result<(), String> {
    let last = f.code.last().ok_or("no code")?;
    if !matches!(last.op, op::RETURN | op::JUMP) {
        return Err("the last instruction must be RETURN or JUMP".into());
    }
    if f.consts.iter().any(|k| k.heap_ref().is_some()) {
        return Err("constants may not refer to heap objects".into());
    }
    for (pc, i) in f.code.iter().enumerate() {
        let fail = |what: &str| Err(format!("pc {pc} ({i:?}): {what}"));
        match i.op {
            op::LOAD_CONST if usize::from(i.bx()) >= f.consts.len() => {
                return fail("constant out of range");
            }
            op::LOAD_MISC if i.b >= 4 => return fail("misc value out of range"),
            op::GET_SLOT if i.c >= 4 => return fail("slot out of range"),
            op::SET_SLOT | op::GET_ENV if i.b >= 4 => return fail("slot out of range"),
            op::SET_ENV if i.a >= 4 => return fail("slot out of range"),
            op::CLOSURE if usize::from(i.b) >= program.functions.len() => {
                return fail("function out of range");
            }
            op::CALL if usize::from(i.a) + usize::from(i.b) > 255 => {
                return fail("arguments beyond the last register");
            }
            op::JUMP | op::JUMP_IF | op::JUMP_IF_NOT => {
                let target = pc.cast_signed() + 1 + isize::from(i.sbx());
                if usize::try_from(target).map_or(true, |t| t >= f.code.len()) {
                    return fail("jump out of range");
                }
            }
            x if usize::from(x) >= op::COUNT => return fail("unknown opcode"),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(code: Vec<Instr>, consts: Vec<Value>) -> Program {
        Program {
            functions: vec![Function {
                name: "main".into(),
                params: 0,
                code,
                consts,
            }],
        }
    }

    #[test]
    fn operands_decode() {
        let i = Instr::with_bx(op::JUMP, 0, (-3_i16).cast_unsigned());
        assert_eq!(i.sbx(), -3);
        assert_eq!(Instr::with_bx(op::LOAD_CONST, 1, 0x1234).bx(), 0x1234);
        assert_eq!(Instr::new(op::ADD_IMM, 0, 0, 0xFF).sc(), -1);
    }

    #[test]
    fn the_verifier_rejects_unsafe_programs() {
        let ret = Instr::new(op::RETURN, 0, 0, 0);
        assert!(program(vec![ret], vec![]).verify().is_ok());
        assert!(program(vec![], vec![]).verify().is_err());
        let fall_off = Instr::new(op::LOAD_INT, 0, 0, 0);
        assert!(program(vec![fall_off], vec![]).verify().is_err());
        let bad_const = Instr::with_bx(op::LOAD_CONST, 0, 1);
        assert!(
            program(vec![bad_const, ret], vec![Value::int(1)])
                .verify()
                .is_err()
        );
        let bad_jump = Instr::with_bx(op::JUMP, 0, 5);
        assert!(program(vec![bad_jump, ret], vec![]).verify().is_err());
        let back = Instr::with_bx(op::JUMP, 0, (-1_i16).cast_unsigned());
        assert!(
            program(vec![back], vec![]).verify().is_ok(),
            "a loop to itself"
        );
        let bad_slot = Instr::new(op::GET_SLOT, 0, 0, 4);
        assert!(program(vec![bad_slot, ret], vec![]).verify().is_err());
        let wide_call = Instr::new(op::CALL, 250, 6, 0);
        assert!(program(vec![wide_call, ret], vec![]).verify().is_err());
        let unknown = Instr::new(200, 0, 0, 0);
        assert!(program(vec![unknown, ret], vec![]).verify().is_err());
    }
}
