//! Dispatch by explicit tail calls (`become`, nightly only). Each handler
//! ends by tail-calling the next instruction's handler, so every handler has
//! its own indirect branch, and the code, `pc` and window stay in argument
//! registers.

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

type Handler<S> = for<'x, 'c, 'a, 'r> fn(
    &'x mut St<'c, 'a, 'r, S>,
    &'a [Op<S>],
    usize,
    &'a Window,
) -> Result<Value, VmError>;

/// The running frame's state that is not passed in registers.
pub(crate) struct St<'c, 'a, 'r, S: RootStack> {
    cx: &'c mut Cx<'a, 'r, S>,
    gc: GcScope<'c>,
    function: u32,
    base: usize,
    consts: &'a [Value],
    stop: usize,
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
    let code = &loaded.tail[function as usize][..];
    let consts = &loaded.program.functions[function as usize].consts[..];
    let w = window(cx.regs, base);
    let mut st = St {
        cx,
        gc,
        function,
        base,
        consts,
        stop,
    };
    (code[0].handler)(&mut st, code, 0, w)
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
            fn $name<'c, 'a, 'r, S: RootStack>(
                st: &mut St<'c, 'a, 'r, S>,
                code: &'a [Op<S>],
                pc: usize,
                w: &'a Window,
            ) -> Result<Value, VmError> {
                let i = code[pc].i;
                st.cx.core.stats.instruction(i.op);
                st.cx.$method(w, st.consts, st.base, i, st.gc.reborrow())?;
                let pc = pc + 1;
                become (code[pc].handler)(st, code, pc, w)
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

fn jump_always<'c, 'a, 'r, S: RootStack>(
    st: &mut St<'c, 'a, 'r, S>,
    code: &'a [Op<S>],
    pc: usize,
    w: &'a Window,
) -> Result<Value, VmError> {
    let i = code[pc].i;
    st.cx.core.stats.instruction(i.op);
    let pc = jump(pc + 1, i.sbx());
    become (code[pc].handler)(st, code, pc, w)
}

fn jump_if<'c, 'a, 'r, S: RootStack>(
    st: &mut St<'c, 'a, 'r, S>,
    code: &'a [Op<S>],
    pc: usize,
    w: &'a Window,
) -> Result<Value, VmError> {
    let i = code[pc].i;
    st.cx.core.stats.instruction(i.op);
    let pc = if truthy(reg(w, i.a).get()) {
        jump(pc + 1, i.sbx())
    } else {
        pc + 1
    };
    become (code[pc].handler)(st, code, pc, w)
}

fn jump_if_not<'c, 'a, 'r, S: RootStack>(
    st: &mut St<'c, 'a, 'r, S>,
    code: &'a [Op<S>],
    pc: usize,
    w: &'a Window,
) -> Result<Value, VmError> {
    let i = code[pc].i;
    st.cx.core.stats.instruction(i.op);
    let pc = if truthy(reg(w, i.a).get()) {
        pc + 1
    } else {
        jump(pc + 1, i.sbx())
    };
    become (code[pc].handler)(st, code, pc, w)
}

fn call<'c, 'a, 'r, S: RootStack>(
    st: &mut St<'c, 'a, 'r, S>,
    code: &'a [Op<S>],
    pc: usize,
    w: &'a Window,
) -> Result<Value, VmError> {
    let i = code[pc].i;
    st.cx.core.stats.instruction(i.op);
    let callee = reg(w, i.a).get();
    if let Some(closure) = callee.to_handle::<Closure>() {
        let (function, base) = st.cx.enter(closure, st.function, pc + 1, st.base, i)?;
        let loaded = st.cx.loaded;
        let code = &loaded.tail[function as usize][..];
        st.function = function;
        st.base = base;
        st.consts = &loaded.program.functions[function as usize].consts;
        let w = window(st.cx.regs, base);
        become (code[0].handler)(st, code, 0, w)
    }
    let v = st.cx.call_native(callee, w, st.base, i, st.gc.reborrow())?;
    reg(w, i.a).set(v);
    let pc = pc + 1;
    become (code[pc].handler)(st, code, pc, w)
}

fn ret<'c, 'a, 'r, S: RootStack>(
    st: &mut St<'c, 'a, 'r, S>,
    code: &'a [Op<S>],
    pc: usize,
    w: &'a Window,
) -> Result<Value, VmError> {
    let i = code[pc].i;
    st.cx.core.stats.instruction(i.op);
    let v = reg(w, i.a).get();
    let Some(caller) = st.cx.leave(v, st.stop) else {
        return Ok(v);
    };
    let loaded = st.cx.loaded;
    let function = caller.function;
    let code = &loaded.tail[function as usize][..];
    st.function = function;
    st.base = caller.base as usize;
    st.consts = &loaded.program.functions[function as usize].consts;
    let w = window(st.cx.regs, st.base);
    let pc = caller.pc as usize;
    become (code[pc].handler)(st, code, pc, w)
}
