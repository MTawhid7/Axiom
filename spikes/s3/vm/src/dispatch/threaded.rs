//! Dispatch by handlers that return the next handler. Programs are decoded
//! once so that each instruction carries its handler; a trampoline calls
//! whatever the previous handler returned.
#![expect(
    clippy::inline_always,
    reason = "stepping must inline into every handler, as the other techniques' does"
)]

use s3_gc::{GcScope, RootStack, Value};

use crate::bytecode::{Instr, Program, op};
use crate::heap::Closure;
use crate::machine::{Cx, VmError, Window, jump, reg, truthy, window};

/// A decoded instruction.
pub(crate) struct Op<S: RootStack> {
    handler: Handler<S>,
    i: Instr,
}

impl<S: RootStack> core::fmt::Debug for Op<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.i.fmt(f)
    }
}

type Handler<S> = for<'c, 'a, 'r> fn(&mut St<'c, 'a, 'r, S>) -> Next<S>;

/// The next handler, or `None` when the run is over.
pub(crate) struct Next<S: RootStack>(Option<Handler<S>>);

/// The running frame, and how the run ends.
pub(crate) struct St<'c, 'a, 'r, S: RootStack> {
    cx: &'c mut Cx<'a, 'r, S>,
    gc: GcScope<'c>,
    function: u32,
    base: usize,
    pc: usize,
    code: &'a [Op<S>],
    consts: &'a [Value],
    w: &'a Window,
    stop: usize,
    outcome: Result<Value, VmError>,
}

impl<S: RootStack> St<'_, '_, '_, S> {
    fn load(&mut self, function: u32, base: usize, pc: usize) {
        let loaded = self.cx.loaded;
        self.function = function;
        self.base = base;
        self.pc = pc;
        self.code = &loaded.threaded[function as usize];
        self.consts = &loaded.program.functions[function as usize].consts;
        self.w = window(self.cx.regs, base);
    }

    /// Step to the next instruction and return its handler.
    #[inline(always)]
    fn next(&mut self) -> Next<S> {
        self.pc += 1;
        Next(Some(self.code[self.pc].handler))
    }

    /// Continue at `pc`, already set.
    #[inline(always)]
    fn resume(&mut self) -> Next<S> {
        Next(Some(self.code[self.pc].handler))
    }

    #[inline(always)]
    fn instr(&mut self) -> Instr {
        let i = self.code[self.pc].i;
        self.cx.core.stats.instruction(i.op);
        i
    }

    fn fail(&mut self, error: VmError) -> Next<S> {
        self.outcome = Err(error);
        Next(None)
    }
}

/// Decode every function of `program`.
pub(crate) fn decode<S: RootStack>(program: &Program) -> Vec<Box<[Op<S>]>> {
    program
        .functions
        .iter()
        .map(|f| {
            f.code
                .iter()
                .map(|&i| Op {
                    handler: handler::<S>(i.op),
                    i,
                })
                .collect()
        })
        .collect()
}

pub(crate) fn run<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    function: u32,
    base: usize,
    stop: usize,
    gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let loaded = cx.loaded;
    let (code, consts, w) = (
        &loaded.threaded[function as usize][..],
        &loaded.program.functions[function as usize].consts[..],
        window(cx.regs, base),
    );
    let mut st = St {
        cx,
        gc,
        function,
        base,
        pc: 0,
        code,
        consts,
        w,
        stop,
        outcome: Ok(Value::UNDEFINED),
    };
    let mut handler = st.code[0].handler;
    loop {
        match handler(&mut st).0 {
            Some(next) => handler = next,
            None => return st.outcome,
        }
    }
}

fn handler<S: RootStack>(opcode: u8) -> Handler<S> {
    match opcode {
        op::LOAD_INT => load_int::<S>,
        op::LOAD_CONST => load_const::<S>,
        op::LOAD_MISC => load_misc::<S>,
        op::MOVE => mov::<S>,
        op::ADD => add::<S>,
        op::SUB => sub::<S>,
        op::MUL => mul::<S>,
        op::DIV => div::<S>,
        op::MOD => rem::<S>,
        op::ADD_IMM => add_imm::<S>,
        op::LT => lt::<S>,
        op::LE => le::<S>,
        op::EQ => eq::<S>,
        op::JUMP => jump_always::<S>,
        op::JUMP_IF => jump_if::<S>,
        op::JUMP_IF_NOT => jump_if_not::<S>,
        op::NEW_OBJECT => new_object::<S>,
        op::GET_SLOT => get_slot::<S>,
        op::SET_SLOT => set_slot::<S>,
        op::NEW_ARRAY => new_array::<S>,
        op::GET_INDEX => get_index::<S>,
        op::SET_INDEX => set_index::<S>,
        op::LENGTH => length::<S>,
        op::CLOSURE => closure::<S>,
        op::GET_ENV => get_env::<S>,
        op::SET_ENV => set_env::<S>,
        op::CALL => call::<S>,
        op::RETURN => ret::<S>,
        _ => unreachable!("verified programs have no unknown opcodes"),
    }
}

/// Handlers for the operations without control flow.
macro_rules! handlers {
    ($($name:ident => $method:ident),+ $(,)?) => {
        $(
            fn $name<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
                let i = st.instr();
                match st.cx.$method(st.w, st.consts, st.base, i, st.gc.reborrow()) {
                    Ok(()) => st.next(),
                    Err(e) => st.fail(e),
                }
            }
        )+
    };
}

handlers! {
    load_int => op_load_int,
    load_const => op_load_const,
    load_misc => op_load_misc,
    mov => op_move,
    add => op_add,
    sub => op_sub,
    mul => op_mul,
    div => op_div,
    rem => op_mod,
    add_imm => op_add_imm,
    lt => op_lt,
    le => op_le,
    eq => op_eq,
    new_object => op_new_object,
    get_slot => op_get_slot,
    set_slot => op_set_slot,
    new_array => op_new_array,
    get_index => op_get_index,
    set_index => op_set_index,
    length => op_length,
    closure => op_closure,
    get_env => op_get_env,
    set_env => op_set_env,
}

fn jump_always<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
    let i = st.instr();
    st.pc = jump(st.pc + 1, i.sbx());
    st.resume()
}

fn jump_if<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
    let i = st.instr();
    if truthy(reg(st.w, i.a).get()) {
        st.pc = jump(st.pc + 1, i.sbx());
        st.resume()
    } else {
        st.next()
    }
}

fn jump_if_not<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
    let i = st.instr();
    if truthy(reg(st.w, i.a).get()) {
        st.next()
    } else {
        st.pc = jump(st.pc + 1, i.sbx());
        st.resume()
    }
}

fn call<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
    let i = st.instr();
    let callee = reg(st.w, i.a).get();
    if let Some(closure) = callee.to_handle::<Closure>() {
        match st.cx.enter(closure, st.function, st.pc + 1, st.base, i) {
            Ok((function, base)) => {
                st.load(function, base, 0);
                st.resume()
            }
            Err(e) => st.fail(e),
        }
    } else {
        match st
            .cx
            .call_native(callee, st.w, st.base, i, st.gc.reborrow())
        {
            Ok(v) => {
                reg(st.w, i.a).set(v);
                st.next()
            }
            Err(e) => st.fail(e),
        }
    }
}

fn ret<S: RootStack>(st: &mut St<'_, '_, '_, S>) -> Next<S> {
    let i = st.instr();
    let v = reg(st.w, i.a).get();
    match st.cx.leave(v, st.stop) {
        None => {
            st.outcome = Ok(v);
            Next(None)
        }
        Some(caller) => {
            st.load(caller.function, caller.base as usize, caller.pc as usize);
            st.resume()
        }
    }
}
