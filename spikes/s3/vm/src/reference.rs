//! The reference evaluator: the oracle for every interpreter configuration.
//!
//! It implements the same bytecode semantics independently and as plainly as
//! possible:
//! - numbers are always `f64`, so the interpreter's integer fast paths are
//!   checked against plain double arithmetic;
//! - the heap is three `Vec`s that never collect or move, so nothing needs
//!   rooting;
//! - every call is a Rust call with its own register vector, and registers a
//!   program must not read are *poisoned*: the ones a callee clobbers, and
//!   those a frame never wrote. Reading one is an error, so a program that
//!   relies on stale registers fails here instead of passing by accident.
//!
//! Builtins and coercion hooks make the same calls, in the same order, as the
//! interpreter's, so even callbacks with side effects must agree.
#![expect(
    clippy::many_single_char_names,
    reason = "an interpreter's idiom: operands a, b, c, x and y"
)]

use std::collections::HashMap;

use s3_gc::Value;

use crate::builtins::Builtin;
use crate::bytecode::{Instr, Program, op};
use crate::digest::Digest;
use crate::machine::VmError;

/// Closure calls deep before a stack overflow.
const MAX_DEPTH: usize = 1000;
const MAX_GAP: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq)]
enum R {
    Num(f64),
    Undef,
    Null,
    Bool(bool),
    Obj(usize),
    Arr(usize),
    Clo(usize),
    Builtin(u8),
    /// A register that must not be read.
    Poison,
}

/// What a reference run produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The result's digest (see [`crate::digest`]), or the error.
    pub result: Result<u64, VmError>,
    /// Objects, arrays and closures allocated: nothing is ever freed.
    pub allocations: usize,
    /// Instructions executed. The interpreter executes exactly these, so
    /// this is the denominator of its time per instruction.
    pub instructions: u64,
}

/// Run `program`'s entry function with primitive `args`.
///
/// # Panics
/// If an argument refers to a heap object, or the program reads a register
/// it must not read (a bug in the program, not in an interpreter).
#[must_use]
pub fn run(program: &Program, args: &[Value]) -> Outcome {
    let mut m = Machine {
        program,
        objects: Vec::new(),
        arrays: Vec::new(),
        closures: Vec::new(),
        depth: 0,
        instructions: 0,
    };
    let args: Vec<R> = args.iter().map(|&a| primitive(a)).collect();
    m.closures.push((0, R::Undef));
    let result = m.call(R::Clo(0), &args).map(|v| m.digest(v));
    Outcome {
        result,
        allocations: m.objects.len() + m.arrays.len() + m.closures.len(),
        instructions: m.instructions,
    }
}

fn primitive(v: Value) -> R {
    if let Some(x) = v.as_f64() {
        R::Num(x)
    } else if let Some(b) = v.as_bool() {
        R::Bool(b)
    } else if let Some(id) = v.as_builtin() {
        R::Builtin(id)
    } else if v == Value::UNDEFINED {
        R::Undef
    } else if v == Value::NULL {
        R::Null
    } else {
        panic!("entry arguments are primitives: {v:?}")
    }
}

struct Machine<'p> {
    program: &'p Program,
    objects: Vec<[R; 4]>,
    arrays: Vec<Vec<R>>,
    closures: Vec<(u8, R)>,
    depth: usize,
    instructions: u64,
}

fn read(regs: &[R], r: usize) -> R {
    match regs[r] {
        R::Poison => panic!("the program read clobbered or unwritten register {r}"),
        v => v,
    }
}

fn truthy(v: R) -> bool {
    match v {
        R::Num(x) => x != 0.0 && !x.is_nan(),
        R::Bool(b) => b,
        R::Undef | R::Null => false,
        R::Obj(_) | R::Arr(_) | R::Clo(_) | R::Builtin(_) => true,
        R::Poison => unreachable!("poison is never read"),
    }
}

fn strict_eq(x: R, y: R) -> bool {
    match (x, y) {
        (R::Num(a), R::Num(b)) => a == b,
        _ => x == y,
    }
}

fn index(v: R) -> Result<usize, VmError> {
    match v {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked: integral and in range"
        )]
        R::Num(x) if x >= 0.0 && x.fract() == 0.0 && x < 4_294_967_295.0 => Ok(x as usize),
        _ => Err(VmError::BadIndex),
    }
}

impl Machine<'_> {
    fn call(&mut self, callee: R, args: &[R]) -> Result<R, VmError> {
        match callee {
            R::Clo(c) => {
                if self.depth >= MAX_DEPTH {
                    return Err(VmError::StackOverflow);
                }
                self.depth += 1;
                let result = self.frame(c, args);
                self.depth -= 1;
                result
            }
            R::Builtin(id) => match Builtin::from_id(id) {
                Some(Builtin::ForEach) => self.for_each(args),
                Some(Builtin::Map) => self.map(args),
                Some(Builtin::SortBy) => self.sort_by(args),
                None => Err(VmError::NotCallable),
            },
            _ => Err(VmError::NotCallable),
        }
    }

    fn coerce_number(&mut self, v: R) -> Result<f64, VmError> {
        match v {
            R::Num(x) => Ok(x),
            R::Bool(b) => Ok(f64::from(u8::from(b))),
            R::Null => Ok(0.0),
            R::Obj(o) => {
                let hook = self.objects[o][3];
                if !matches!(hook, R::Clo(_) | R::Builtin(_)) {
                    return Ok(f64::NAN);
                }
                match self.call(hook, &[v])? {
                    R::Obj(_) | R::Arr(_) | R::Clo(_) => Err(VmError::NotPrimitive),
                    primitive => self.coerce_number(primitive),
                }
            }
            R::Undef | R::Arr(_) | R::Clo(_) | R::Builtin(_) => Ok(f64::NAN),
            R::Poison => unreachable!("poison is never read"),
        }
    }

    fn arith(&mut self, x: R, y: R, f: fn(f64, f64) -> f64) -> Result<R, VmError> {
        let a = self.coerce_number(x)?;
        let b = self.coerce_number(y)?;
        Ok(R::Num(f(a, b)))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one arm per opcode, like the interpreter"
    )]
    fn frame(&mut self, closure: usize, args: &[R]) -> Result<R, VmError> {
        let (function, env) = self.closures[closure];
        let f = &self.program.functions[usize::from(function)];
        let mut regs = vec![R::Poison; 256];
        regs[..args.len()].copy_from_slice(args);
        for r in regs.iter_mut().take(usize::from(f.params)).skip(args.len()) {
            *r = R::Undef;
        }
        let mut pc = 0;
        loop {
            let Instr { op, a, b, c } = f.code[pc];
            let (a, b, c) = (usize::from(a), usize::from(b), usize::from(c));
            let i = f.code[pc];
            pc += 1;
            self.instructions += 1;
            match op {
                op::LOAD_INT => regs[a] = R::Num(f64::from(i.sbx())),
                op::LOAD_CONST => regs[a] = primitive(f.consts[usize::from(i.bx())]),
                op::LOAD_MISC => regs[a] = [R::Undef, R::Null, R::Bool(false), R::Bool(true)][b],
                op::MOVE => regs[a] = read(&regs, b),
                op::ADD => regs[a] = self.arith(read(&regs, b), read(&regs, c), |x, y| x + y)?,
                op::SUB => regs[a] = self.arith(read(&regs, b), read(&regs, c), |x, y| x - y)?,
                op::MUL => regs[a] = self.arith(read(&regs, b), read(&regs, c), |x, y| x * y)?,
                op::DIV => regs[a] = self.arith(read(&regs, b), read(&regs, c), |x, y| x / y)?,
                op::MOD => regs[a] = self.arith(read(&regs, b), read(&regs, c), |x, y| x % y)?,
                op::ADD_IMM => {
                    regs[a] = R::Num(self.coerce_number(read(&regs, b))? + f64::from(i.sc()));
                }
                op::LT | op::LE => {
                    let x = self.coerce_number(read(&regs, b))?;
                    let y = self.coerce_number(read(&regs, c))?;
                    regs[a] = R::Bool(if op == op::LT { x < y } else { x <= y });
                }
                op::EQ => regs[a] = R::Bool(strict_eq(read(&regs, b), read(&regs, c))),
                op::JUMP => pc = pc.wrapping_add_signed(isize::from(i.sbx())),
                op::JUMP_IF | op::JUMP_IF_NOT => {
                    if truthy(read(&regs, a)) == (op == op::JUMP_IF) {
                        pc = pc.wrapping_add_signed(isize::from(i.sbx()));
                    }
                }
                op::NEW_OBJECT => {
                    self.objects.push([R::Undef; 4]);
                    regs[a] = R::Obj(self.objects.len() - 1);
                }
                op::GET_SLOT => match read(&regs, b) {
                    R::Obj(o) => regs[a] = self.objects[o][c],
                    _ => return Err(VmError::NotObject),
                },
                op::SET_SLOT => match read(&regs, a) {
                    R::Obj(o) => self.objects[o][b] = read(&regs, c),
                    _ => return Err(VmError::NotObject),
                },
                op::NEW_ARRAY => {
                    let len = index(read(&regs, b))?;
                    if len > MAX_GAP {
                        return Err(VmError::BadIndex);
                    }
                    self.arrays.push(vec![R::Undef; len]);
                    regs[a] = R::Arr(self.arrays.len() - 1);
                }
                op::GET_INDEX => {
                    let R::Arr(array) = read(&regs, b) else {
                        return Err(VmError::NotArray);
                    };
                    let at = index(read(&regs, c))?;
                    regs[a] = self.arrays[array].get(at).copied().unwrap_or(R::Undef);
                }
                op::SET_INDEX => {
                    let R::Arr(array) = read(&regs, a) else {
                        return Err(VmError::NotArray);
                    };
                    let at = index(read(&regs, b))?;
                    let value = read(&regs, c);
                    let items = &mut self.arrays[array];
                    if at < items.len() {
                        items[at] = value;
                    } else if at - items.len() > MAX_GAP {
                        return Err(VmError::BadIndex);
                    } else {
                        items.resize(at, R::Undef);
                        items.push(value);
                    }
                }
                op::LENGTH => match read(&regs, b) {
                    #[expect(clippy::cast_precision_loss, reason = "lengths are small")]
                    R::Arr(array) => regs[a] = R::Num(self.arrays[array].len() as f64),
                    _ => return Err(VmError::NotArray),
                },
                op::CLOSURE => {
                    let env = read(&regs, c);
                    self.closures
                        .push((u8::try_from(b).expect("a u8 operand"), env));
                    regs[a] = R::Clo(self.closures.len() - 1);
                }
                op::GET_ENV => match env {
                    R::Obj(o) => regs[a] = self.objects[o][b],
                    _ => return Err(VmError::NotObject),
                },
                op::SET_ENV => match env {
                    R::Obj(o) => self.objects[o][a] = read(&regs, b),
                    _ => return Err(VmError::NotObject),
                },
                op::CALL => {
                    let callee = read(&regs, a);
                    let args: Vec<R> = (a + 1..=a + b).map(|r| read(&regs, r)).collect();
                    let result = self.call(callee, &args)?;
                    // The callee's frame started at R[a+1]: everything above R[a] is gone.
                    for r in &mut regs[a + 1..] {
                        *r = R::Poison;
                    }
                    regs[a] = result;
                }
                op::RETURN => return Ok(read(&regs, a)),
                _ => unreachable!("verified programs have no unknown opcodes"),
            }
        }
    }

    fn array_arg(args: &[R], k: usize) -> Result<usize, VmError> {
        match args.get(k) {
            Some(&R::Arr(array)) => Ok(array),
            _ => Err(VmError::NotArray),
        }
    }

    fn for_each(&mut self, args: &[R]) -> Result<R, VmError> {
        let array = Self::array_arg(args, 0)?;
        let f = args.get(1).copied().unwrap_or(R::Undef);
        let len = self.arrays[array].len();
        for i in 0..len {
            let Some(&element) = self.arrays[array].get(i) else {
                continue;
            };
            #[expect(clippy::cast_precision_loss, reason = "small indices")]
            self.call(f, &[element, R::Num(i as f64)])?;
        }
        Ok(R::Undef)
    }

    fn map(&mut self, args: &[R]) -> Result<R, VmError> {
        let array = Self::array_arg(args, 0)?;
        let f = args.get(1).copied().unwrap_or(R::Undef);
        let len = self.arrays[array].len();
        self.arrays.push(vec![R::Undef; len]);
        let result = self.arrays.len() - 1;
        for i in 0..len {
            let Some(&element) = self.arrays[array].get(i) else {
                continue;
            };
            #[expect(clippy::cast_precision_loss, reason = "small indices")]
            let mapped = self.call(f, &[element, R::Num(i as f64)])?;
            self.arrays[result][i] = mapped;
        }
        Ok(R::Arr(result))
    }

    /// The interpreter's bottom-up merge sort, call for call.
    fn sort_by(&mut self, args: &[R]) -> Result<R, VmError> {
        let array = Self::array_arg(args, 0)?;
        let compare = args.get(1).copied().unwrap_or(R::Undef);
        let len = self.arrays[array].len();
        if len < 2 {
            return Ok(R::Arr(array));
        }
        self.arrays.push(vec![R::Undef; len]);
        let scratch = self.arrays.len() - 1;
        let element = |m: &Self, from: usize, i: usize| {
            m.arrays[from].get(i).copied().ok_or(VmError::BadIndex)
        };
        let mut in_array = true;
        let mut width = 1;
        while width < len {
            let (from, to) = if in_array {
                (array, scratch)
            } else {
                (scratch, array)
            };
            let mut lo = 0;
            while lo < len {
                let mid = (lo + width).min(len);
                let hi = (lo + 2 * width).min(len);
                let (mut i, mut j) = (lo, mid);
                for k in lo..hi {
                    let take_left = if i < mid && j < hi {
                        let (x, y) = (element(self, from, i)?, element(self, from, j)?);
                        let order = self.call(compare, &[x, y])?;
                        let order = self.coerce_number(order)?;
                        order <= 0.0 || order.is_nan()
                    } else {
                        i < mid
                    };
                    let v = element(self, from, if take_left { i } else { j })?;
                    *self.arrays[to].get_mut(k).ok_or(VmError::BadIndex)? = v;
                    if take_left {
                        i += 1;
                    } else {
                        j += 1;
                    }
                }
                lo = hi;
            }
            in_array = !in_array;
            width *= 2;
        }
        if !in_array {
            let sorted = self.arrays[scratch].clone();
            let items = &mut self.arrays[array];
            let n = sorted.len().min(items.len());
            items[..n].copy_from_slice(&sorted[..n]);
        }
        Ok(R::Arr(array))
    }

    /// The same walk as [`crate::heap::digest`].
    fn digest(&self, value: R) -> u64 {
        let mut d = Digest::default();
        let mut seen: HashMap<(u8, usize), usize> = HashMap::new();
        let mut stack = vec![value];
        while let Some(v) = stack.pop() {
            let key = match v {
                R::Num(x) => {
                    d.number(x);
                    continue;
                }
                R::Bool(b) => {
                    d.boolean(b);
                    continue;
                }
                R::Builtin(id) => {
                    d.builtin(id);
                    continue;
                }
                R::Undef => {
                    d.undefined();
                    continue;
                }
                R::Null => {
                    d.null();
                    continue;
                }
                R::Obj(o) => (0, o),
                R::Arr(a) => (1, a),
                R::Clo(c) => (2, c),
                R::Poison => unreachable!("poison is never read"),
            };
            let n = seen.len();
            if let Some(&first) = seen.get(&key) {
                d.backref(first);
                continue;
            }
            seen.insert(key, n);
            match key {
                (0, o) => {
                    d.object();
                    stack.extend(self.objects[o].iter().rev());
                }
                (1, a) => {
                    d.array(self.arrays[a].len());
                    stack.extend(self.arrays[a].iter().rev());
                }
                (_, c) => {
                    let (function, env) = self.closures[c];
                    d.closure(u32::from(function));
                    stack.push(env);
                }
            }
        }
        d.finish()
    }
}
