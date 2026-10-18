//! The machine: state, calls, collection points, and the semantics of every
//! operation that isn't control flow. The dispatch techniques
//! ([`crate::dispatch`]) share these, so they differ only in how they find
//! and run the next instruction.
#![expect(
    clippy::many_single_char_names,
    reason = "an interpreter's idiom: w(indow), i(nstruction), operands x and y, numbers a and b"
)]
#![expect(
    clippy::inline_always,
    reason = "the operations must inline into every technique's handlers, or the techniques would differ by call overhead instead of by dispatch"
)]
#![expect(
    clippy::unused_self,
    clippy::unnecessary_wraps,
    reason = "every operation has the same signature, so dispatch techniques can call them uniformly"
)]

use core::cell::Cell;
use core::fmt;

use s3_gc::count::{self, Context};
use s3_gc::{GcScope, GcToken, Handle, Metrics, Pacing, RootStack, Value};

use crate::builtins;
use crate::bytecode::{Instr, MISC, Program};
use crate::dispatch::{self, Technique};
use crate::heap::{Array, Closure, Object, Registers, VmHeap, digest};
use crate::stats::Stats;

/// The callee slot plus 256 registers: what one frame sees.
pub(crate) const WINDOW: usize = 257;
pub(crate) type Window = [Cell<Value>; WINDOW];
/// Registers in the register file: 512 KiB.
pub const REGISTERS: usize = 1 << 16;
/// Bytecode frames before a stack overflow.
pub const MAX_FRAMES: usize = 10_000;
/// Nested native calls into bytecode (builtins and hooks) before a stack
/// overflow. Each one is a Rust stack frame.
pub const MAX_NATIVE_DEPTH: usize = 128;
/// How far past its end an array may be written.
const MAX_GAP: usize = 1 << 20;

/// What can go wrong at run time. One byte, so results stay in registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmError {
    NotCallable,
    NotObject,
    NotArray,
    NotClosure,
    /// A coercion hook returned an object.
    NotPrimitive,
    /// A negative, fractional or far out-of-range array index or length.
    BadIndex,
    StackOverflow,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// A caller's state, saved while its callee runs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub(crate) function: u32,
    pub(crate) pc: u32,
    pub(crate) base: u32,
    /// The caller's register for the result.
    pub(crate) dst: u8,
}

/// A program, verified and decoded for every dispatch technique.
#[derive(Debug)]
pub struct Loaded<S: RootStack> {
    pub(crate) program: Program,
    pub(crate) threaded: Vec<Box<[dispatch::threaded::Op<S>]>>,
    #[cfg(feature = "nightly-tail")]
    pub(crate) tail: Vec<Box<[dispatch::tail::Op<S>]>>,
}

impl<S: RootStack> Loaded<S> {
    /// # Errors
    /// If the program fails verification.
    pub fn new(program: Program) -> Result<Self, String> {
        program.verify()?;
        Ok(Self {
            threaded: dispatch::threaded::decode(&program),
            #[cfg(feature = "nightly-tail")]
            tail: dispatch::tail::decode(&program),
            program,
        })
    }

    #[must_use]
    pub const fn program(&self) -> &Program {
        &self.program
    }
}

/// Everything but the register file.
#[derive(Debug)]
pub(crate) struct Core<'r, S: RootStack> {
    pub(crate) heap: VmHeap<'r, S>,
    pub(crate) frames: Vec<Frame>,
    /// Registers below this may hold values: the traced region.
    pub(crate) high_water: usize,
    pub(crate) native_depth: usize,
    pub(crate) stats: Stats,
}

/// One agent's interpreter: a heap, a register file, and a frame stack.
pub struct Vm<'r, S: RootStack> {
    core: Core<'r, S>,
    regs: Box<[Cell<Value>]>,
}

impl<'r, S: RootStack> Vm<'r, S> {
    /// A machine whose builtins root values on `roots`.
    #[must_use]
    pub fn new(roots: &'r S, pacing: Pacing) -> Self {
        Self {
            core: Core {
                heap: VmHeap::new(roots, pacing),
                frames: Vec::new(),
                high_water: 0,
                native_depth: 0,
                stats: Stats::default(),
            },
            regs: (0..REGISTERS)
                .map(|_| Cell::new(Value::UNDEFINED))
                .collect(),
        }
    }

    /// Run the entry function with `args`, which must not refer to heap
    /// objects.
    ///
    /// # Errors
    /// A run-time error from the program.
    pub fn run(
        &mut self,
        loaded: &Loaded<S>,
        technique: Technique,
        args: &[Value],
    ) -> Result<Value, VmError> {
        assert!(
            args.iter().all(|a| a.heap_ref().is_none()),
            "entry arguments are primitives"
        );
        let mut token = GcToken::new();
        let mut gc = GcScope::new(&mut token);
        let mut cx = Cx {
            core: &mut self.core,
            regs: &self.regs,
            loaded,
            technique,
        };
        cx.safepoint(gc.reborrow());
        let main = cx.core.heap.alloc(Closure {
            function: 0,
            env: Value::UNDEFINED,
        });
        let result = cx.call_value(main.to_value(), args, 0, gc.reborrow());
        debug_assert!(cx.core.frames.is_empty() || result.is_err());
        cx.core.frames.clear();
        cx.core.native_depth = 0;
        result
    }

    /// The canonical digest of `value` and everything it reaches.
    #[must_use]
    pub fn digest(&self, value: Value) -> u64 {
        digest(&self.core.heap, value)
    }

    /// A full collection. Registers keep their values alive, so pass the
    /// result of [`run`](Self::run) through here only to see it relocated:
    /// it is not rooted.
    pub fn collect(&mut self) {
        let traced = &self.regs[..self.core.high_water];
        self.core.heap.collect(&mut Registers(traced));
    }

    /// Forget every register, so the next collection frees everything.
    pub fn clear_registers(&mut self) {
        for r in &self.regs[..self.core.high_water] {
            r.set(Value::UNDEFINED);
        }
        self.core.high_water = 0;
    }

    #[must_use]
    pub const fn metrics(&self) -> Metrics {
        self.core.heap.core.metrics()
    }

    #[must_use]
    pub const fn heap(&self) -> &VmHeap<'r, S> {
        &self.core.heap
    }

    /// Counts since the last call, which resets them. All zero unless built
    /// with the `count` feature.
    pub fn take_stats(&mut self) -> Stats {
        core::mem::take(&mut self.core.stats)
    }
}

impl<S: RootStack> fmt::Debug for Vm<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vm")
            .field("frames", &self.core.frames.len())
            .field("high_water", &self.core.high_water)
            .finish_non_exhaustive()
    }
}

/// What every operation works on: the machine's state, borrowed for one run.
/// The register file is shared (`Cell`s), so windows into it stay valid
/// while the core is borrowed mutably, across calls and collections alike.
pub(crate) struct Cx<'a, 'r, S: RootStack> {
    pub(crate) core: &'a mut Core<'r, S>,
    pub(crate) regs: &'a [Cell<Value>],
    pub(crate) loaded: &'a Loaded<S>,
    pub(crate) technique: Technique,
}

/// The window of the frame whose registers start at `base`: `w[0]` is the
/// running closure, `w[1 + r]` is register `r`.
#[inline]
pub(crate) fn window(regs: &[Cell<Value>], base: usize) -> &Window {
    <&Window>::try_from(&regs[base - 1..base - 1 + WINDOW]).expect("a window is WINDOW registers")
}

/// Register `r` of a window. Never bounds-checked: `r` is a `u8`.
#[inline(always)]
pub(crate) fn reg(w: &Window, r: u8) -> &Cell<Value> {
    &w[usize::from(r) + 1]
}

/// The target of a jump at the instruction before `pc`.
#[inline(always)]
pub(crate) fn jump(pc: usize, offset: i16) -> usize {
    pc.wrapping_add_signed(isize::from(offset))
}

/// JavaScript's `ToBoolean` on everything but heap objects, which are true.
#[inline(always)]
pub(crate) fn truthy(v: Value) -> bool {
    if v == Value::TRUE {
        return true;
    }
    if v == Value::FALSE {
        return false;
    }
    if let Some(i) = v.as_int() {
        return i != 0;
    }
    if v.is_double() {
        let x = f64::from_bits(v.to_bits());
        return x != 0.0 && !x.is_nan();
    }
    !(v == Value::UNDEFINED || v == Value::NULL)
}

/// Strict equality (`===`).
#[inline]
pub(crate) fn strict_eq(x: Value, y: Value) -> bool {
    match (x.as_f64(), y.as_f64()) {
        (Some(a), Some(b)) => a == b,
        (None, None) => x.identity() == y.identity(),
        _ => false,
    }
}

pub(crate) fn callable(v: Value) -> bool {
    v.as_builtin().is_some() || v.to_handle::<Closure>().is_some()
}

#[derive(Clone, Copy)]
enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Arith {
    fn apply(self, x: f64, y: f64) -> f64 {
        match self {
            Self::Add => x + y,
            Self::Sub => x - y,
            Self::Mul => x * y,
            Self::Div => x / y,
            Self::Mod => x % y,
        }
    }
}

/// A number as an array index or length.
fn index(v: Value) -> Result<usize, VmError> {
    if let Some(i) = v.as_int() {
        return usize::try_from(i).map_err(|_| VmError::BadIndex);
    }
    match v.as_f64() {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked: integral and in range"
        )]
        Some(x) if x >= 0.0 && x.fract() == 0.0 && x < 4_294_967_295.0 => Ok(x as usize),
        _ => Err(VmError::BadIndex),
    }
}

impl<S: RootStack> Cx<'_, '_, S> {
    /// Collect if the allocation budget is spent. Taking a `GcScope` makes
    /// every caller give up its bound handles.
    #[inline]
    pub(crate) fn safepoint(&mut self, _gc: GcScope<'_>) {
        if self.core.heap.core.wants_collection() {
            self.collect();
        }
    }

    #[cold]
    #[inline(never)]
    fn collect(&mut self) {
        let traced = &self.regs[..self.core.high_water];
        self.core.heap.collect(&mut Registers(traced));
    }

    // ----- Calls -----

    /// Check that a frame can start at `base`, and account for its window.
    #[inline]
    pub(crate) fn open_frame(&mut self, base: usize) -> Result<(), VmError> {
        if base + WINDOW > self.regs.len() || self.core.frames.len() >= MAX_FRAMES {
            return Err(VmError::StackOverflow);
        }
        self.core.high_water = self.core.high_water.max(base + WINDOW - 1);
        Ok(())
    }

    /// Enter closure `callee` from `CALL a, b` in the frame of `function` at
    /// `base`, about to resume at `pc`. Returns the callee's function and base.
    #[inline]
    pub(crate) fn enter(
        &mut self,
        callee: Handle<'_, Closure>,
        function: u32,
        pc: usize,
        base: usize,
        i: Instr,
    ) -> Result<(u32, usize), VmError> {
        let new_base = base + usize::from(i.a) + 1;
        self.open_frame(new_base)?;
        let target = self.core.heap.closures[callee].function;
        let params = self.loaded.program.functions[target as usize].params;
        for missing in i.b..params {
            self.regs[new_base + usize::from(missing)].set(Value::UNDEFINED);
        }
        self.core.frames.push(Frame {
            function,
            pc: u32::try_from(pc).expect("small code"),
            base: u32::try_from(base).expect("small register file"),
            dst: i.a,
        });
        self.core.stats.bytecode_call();
        Ok((target, new_base))
    }

    /// Return `v` from the running frame. `None` means the run that started
    /// at depth `stop` is over; otherwise the caller's frame, with `v` stored.
    #[inline]
    pub(crate) fn leave(&mut self, v: Value, stop: usize) -> Option<Frame> {
        if self.core.frames.len() == stop {
            return None;
        }
        let frame = self.core.frames.pop().expect("a caller below stop");
        self.regs[frame.base as usize + usize::from(frame.dst)].set(v);
        Some(frame)
    }

    /// Call a builtin from `CALL a, b`, or fail if `callee` isn't callable.
    pub(crate) fn call_native(
        &mut self,
        callee: Value,
        w: &Window,
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<Value, VmError> {
        let id = callee.as_builtin().ok_or(VmError::NotCallable)?;
        let mut args = [Value::UNDEFINED; 4];
        let first = usize::from(i.a) + 2;
        for (k, arg) in args.iter_mut().enumerate().take(usize::from(i.b)) {
            *arg = w[first + k].get();
        }
        let n = usize::from(i.b).min(args.len());
        // A builtin's callees start at R[a], over the builtin and its arguments.
        builtins::call(self, id, &args[..n], base + usize::from(i.a), gc)
    }

    /// Call `f` with `args` from native code: a builtin, the coercion hook, or
    /// the entry point. Its frame starts above register `top`, where `f`
    /// goes. Runs to completion with the current dispatch technique.
    pub(crate) fn call_value(
        &mut self,
        f: Value,
        args: &[Value],
        top: usize,
        mut gc: GcScope<'_>,
    ) -> Result<Value, VmError> {
        if let Some(id) = f.as_builtin() {
            return builtins::call(self, id, args, top, gc);
        }
        let closure = f.to_handle::<Closure>().ok_or(VmError::NotCallable)?;
        if self.core.native_depth >= MAX_NATIVE_DEPTH {
            return Err(VmError::StackOverflow);
        }
        let base = top + 1;
        self.open_frame(base)?;
        self.regs[top].set(f);
        for (k, &arg) in args.iter().enumerate() {
            self.regs[base + k].set(arg);
        }
        let function = self.core.heap.closures[closure.bind(gc.nogc())].function;
        let params = usize::from(self.loaded.program.functions[function as usize].params);
        for k in args.len()..params {
            self.regs[base + k].set(Value::UNDEFINED);
        }
        self.core.native_depth += 1;
        self.core.stats.native_call();
        let previous = count::set_context(Context::Interpreter);
        let stop = self.core.frames.len();
        let result = match self.technique {
            Technique::Match => dispatch::matched::run(self, function, base, stop, gc.reborrow()),
            Technique::Table => dispatch::table::run(self, function, base, stop, gc.reborrow()),
            Technique::Threaded => {
                dispatch::threaded::run(self, function, base, stop, gc.reborrow())
            }
            #[cfg(feature = "nightly-tail")]
            Technique::Tail => dispatch::tail::run(self, function, base, stop, gc.reborrow()),
        };
        count::set_context(previous);
        self.core.native_depth -= 1;
        result
    }

    // ----- Numbers -----

    /// JavaScript's `ToNumber`, with objects converted through the closure in
    /// slot 3, called above the frame at `base` so it can't clobber it.
    pub(crate) fn coerce_number(
        &mut self,
        v: Value,
        base: usize,
        mut gc: GcScope<'_>,
    ) -> Result<f64, VmError> {
        if let Some(x) = v.as_f64() {
            return Ok(x);
        }
        if let Some(b) = v.as_bool() {
            return Ok(f64::from(u8::from(b)));
        }
        if v == Value::NULL {
            return Ok(0.0);
        }
        if let Some(object) = v.to_handle::<Object>() {
            let hook = self.core.heap.objects[object.bind(gc.nogc())].slots[3];
            if callable(hook) {
                self.core.stats.coercion();
                let previous = count::set_context(Context::Coercion);
                let result = self.call_value(hook, &[v], base + WINDOW - 1, gc.reborrow());
                count::set_context(previous);
                let primitive = result?;
                if primitive.heap_ref().is_some() {
                    return Err(VmError::NotPrimitive);
                }
                return self.coerce_number(primitive, base, gc);
            }
        }
        // undefined, arrays, closures, builtins, and objects without a hook.
        Ok(f64::NAN)
    }

    /// Arithmetic on anything but two integers or two doubles.
    #[cold]
    #[inline(never)]
    fn arith_slow(
        &mut self,
        w: &Window,
        base: usize,
        i: Instr,
        op: Arith,
        mut gc: GcScope<'_>,
    ) -> Result<Value, VmError> {
        let x = self.coerce_number(reg(w, i.b).get(), base, gc.reborrow())?;
        // Read the other operand only now: the hook may have collected.
        let y = self.coerce_number(reg(w, i.c).get(), base, gc)?;
        Ok(Value::number(op.apply(x, y)))
    }

    #[cold]
    #[inline(never)]
    fn compare_slow(
        &mut self,
        w: &Window,
        base: usize,
        i: Instr,
        mut gc: GcScope<'_>,
    ) -> Result<(f64, f64), VmError> {
        let x = self.coerce_number(reg(w, i.b).get(), base, gc.reborrow())?;
        let y = self.coerce_number(reg(w, i.c).get(), base, gc)?;
        Ok((x, y))
    }

    #[inline(always)]
    fn doubles(x: Value, y: Value) -> Option<(f64, f64)> {
        (x.is_double() && y.is_double())
            .then(|| (f64::from_bits(x.to_bits()), f64::from_bits(y.to_bits())))
    }

    // ----- The operations. All take the same arguments, so dispatch
    // techniques can call them uniformly. -----

    #[inline(always)]
    pub(crate) fn op_load_int(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        reg(w, i.a).set(Value::int(i32::from(i.sbx())));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_load_const(
        &mut self,
        w: &Window,
        consts: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        reg(w, i.a).set(consts[usize::from(i.bx())]);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_load_misc(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        reg(w, i.a).set(MISC[usize::from(i.b & 3)]);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_move(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        reg(w, i.a).set(reg(w, i.b).get());
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_add(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = if let (Some(a), Some(b)) = (x.as_int(), y.as_int()) {
            a.checked_add(b)
                .map_or_else(|| Value::number(f64::from(a) + f64::from(b)), Value::int)
        } else if let Some((a, b)) = Self::doubles(x, y) {
            Value::number(a + b)
        } else {
            self.arith_slow(w, base, i, Arith::Add, gc)?
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_sub(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = if let (Some(a), Some(b)) = (x.as_int(), y.as_int()) {
            a.checked_sub(b)
                .map_or_else(|| Value::number(f64::from(a) - f64::from(b)), Value::int)
        } else if let Some((a, b)) = Self::doubles(x, y) {
            Value::number(a - b)
        } else {
            self.arith_slow(w, base, i, Arith::Sub, gc)?
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_mul(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = if let (Some(a), Some(b)) = (x.as_int(), y.as_int()) {
            match a.checked_mul(b) {
                // 0 times a negative number is -0, which only a double holds.
                Some(0) if a < 0 || b < 0 => Value::number(-0.0),
                Some(product) => Value::int(product),
                None => Value::number(f64::from(a) * f64::from(b)),
            }
        } else if let Some((a, b)) = Self::doubles(x, y) {
            Value::number(a * b)
        } else {
            self.arith_slow(w, base, i, Arith::Mul, gc)?
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_div(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = match (x.as_int(), y.as_int()) {
            // Exact integer quotients stay integers, except -0.
            (Some(a), Some(b)) if b != 0 && a.checked_rem(b) == Some(0) && !(a == 0 && b < 0) => a
                .checked_div(b)
                .map_or_else(|| Value::number(f64::from(a) / f64::from(b)), Value::int),
            (Some(a), Some(b)) => Value::number(f64::from(a) / f64::from(b)),
            _ => match Self::doubles(x, y) {
                Some((a, b)) => Value::number(a / b),
                None => self.arith_slow(w, base, i, Arith::Div, gc)?,
            },
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_mod(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = match (x.as_int(), y.as_int()) {
            // A zero remainder of a negative dividend is -0.
            (Some(a), Some(b)) => match a.checked_rem(b) {
                Some(0) if a < 0 => Value::number(-0.0),
                Some(r) => Value::int(r),
                None => Value::number(f64::from(a) % f64::from(b)),
            },
            _ => match Self::doubles(x, y) {
                Some((a, b)) => Value::number(a % b),
                None => self.arith_slow(w, base, i, Arith::Mod, gc)?,
            },
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_add_imm(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let x = reg(w, i.b).get();
        let n = i32::from(i.sc());
        let v = if let Some(a) = x.as_int() {
            a.checked_add(n)
                .map_or_else(|| Value::number(f64::from(a) + f64::from(n)), Value::int)
        } else if x.is_double() {
            Value::number(f64::from_bits(x.to_bits()) + f64::from(n))
        } else {
            Value::number(self.coerce_number(x, base, gc)? + f64::from(n))
        };
        reg(w, i.a).set(v);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_lt(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = if let (Some(a), Some(b)) = (x.as_int(), y.as_int()) {
            a < b
        } else if let Some((a, b)) = Self::doubles(x, y) {
            a < b
        } else {
            let (a, b) = self.compare_slow(w, base, i, gc)?;
            a < b
        };
        reg(w, i.a).set(Value::bool(v));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_le(
        &mut self,
        w: &Window,
        _: &[Value],
        base: usize,
        i: Instr,
        gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        let v = if let (Some(a), Some(b)) = (x.as_int(), y.as_int()) {
            a <= b
        } else if let Some((a, b)) = Self::doubles(x, y) {
            a <= b
        } else {
            let (a, b) = self.compare_slow(w, base, i, gc)?;
            a <= b
        };
        reg(w, i.a).set(Value::bool(v));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_eq(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let (x, y) = (reg(w, i.b).get(), reg(w, i.c).get());
        reg(w, i.a).set(Value::bool(strict_eq(x, y)));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_new_object(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        mut gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        self.safepoint(gc.reborrow());
        let object = self.core.heap.alloc(Object {
            slots: [Value::UNDEFINED; 4],
        });
        reg(w, i.a).set(object.to_value());
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_get_slot(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let object = reg(w, i.b)
            .get()
            .to_handle::<Object>()
            .ok_or(VmError::NotObject)?;
        reg(w, i.a).set(self.core.heap.objects[object].slots[usize::from(i.c & 3)]);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_set_slot(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let object = reg(w, i.a)
            .get()
            .to_handle::<Object>()
            .ok_or(VmError::NotObject)?;
        self.core.heap.objects[object].slots[usize::from(i.b & 3)] = reg(w, i.c).get();
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_new_array(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        mut gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        let len = index(reg(w, i.b).get())?;
        if len > MAX_GAP {
            return Err(VmError::BadIndex);
        }
        self.safepoint(gc.reborrow());
        let array = self.core.heap.alloc(Array {
            items: vec![Value::UNDEFINED; len],
        });
        reg(w, i.a).set(array.to_value());
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_get_index(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let array = reg(w, i.b)
            .get()
            .to_handle::<Array>()
            .ok_or(VmError::NotArray)?;
        let at = index(reg(w, i.c).get())?;
        let items = &self.core.heap.arrays[array].items;
        reg(w, i.a).set(items.get(at).copied().unwrap_or(Value::UNDEFINED));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_set_index(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let array = reg(w, i.a)
            .get()
            .to_handle::<Array>()
            .ok_or(VmError::NotArray)?;
        let at = index(reg(w, i.b).get())?;
        let value = reg(w, i.c).get();
        let items = &mut self.core.heap.arrays[array].items;
        if let Some(slot) = items.get_mut(at) {
            *slot = value;
        } else if at - items.len() > MAX_GAP {
            return Err(VmError::BadIndex);
        } else {
            items.resize(at, Value::UNDEFINED);
            items.push(value);
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_length(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let array = reg(w, i.b)
            .get()
            .to_handle::<Array>()
            .ok_or(VmError::NotArray)?;
        let len = self.core.heap.arrays[array].items.len();
        reg(w, i.a).set(Value::int(
            i32::try_from(len).map_err(|_| VmError::BadIndex)?,
        ));
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_closure(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        mut gc: GcScope<'_>,
    ) -> Result<(), VmError> {
        self.safepoint(gc.reborrow());
        // Read the environment after the safepoint, which may have moved it.
        let env = reg(w, i.c).get();
        let closure = self.core.heap.alloc(Closure {
            function: u32::from(i.b),
            env,
        });
        reg(w, i.a).set(closure.to_value());
        Ok(())
    }

    /// The running closure's environment object.
    #[inline(always)]
    fn env(&self, w: &Window) -> Result<Handle<'static, Object>, VmError> {
        let closure = w[0]
            .get()
            .to_handle::<Closure>()
            .ok_or(VmError::NotClosure)?;
        self.core.heap.closures[closure]
            .env
            .to_handle::<Object>()
            .ok_or(VmError::NotObject)
    }

    #[inline(always)]
    pub(crate) fn op_get_env(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let env = self.env(w)?;
        reg(w, i.a).set(self.core.heap.objects[env].slots[usize::from(i.b & 3)]);
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn op_set_env(
        &mut self,
        w: &Window,
        _: &[Value],
        _: usize,
        i: Instr,
        _: GcScope<'_>,
    ) -> Result<(), VmError> {
        let env = self.env(w)?;
        self.core.heap.objects[env].slots[usize::from(i.a & 3)] = reg(w, i.b).get();
        Ok(())
    }
}
