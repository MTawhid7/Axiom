//! Dispatch through a table of handler functions, indexed by opcode. Each
//! handler returns to the loop, and the frame's state lives in a struct.

use core::marker::PhantomData;

use s3_gc::{GcScope, RootStack, Value};

use crate::bytecode::{Instr, op};
use crate::heap::Closure;
use crate::machine::{Cx, VmError, Window, jump, reg, truthy, window};

/// The running frame, and how the run ends.
pub(crate) struct St<'c, 'a, 'r, S: RootStack> {
    cx: &'c mut Cx<'a, 'r, S>,
    gc: GcScope<'c>,
    function: u32,
    base: usize,
    pc: usize,
    code: &'a [Instr],
    consts: &'a [Value],
    w: &'a Window,
    stop: usize,
    outcome: Result<Value, VmError>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    Next,
    Stop,
}

type Handler<S> = for<'c, 'a, 'r> fn(&mut St<'c, 'a, 'r, S>, Instr) -> Flow;

impl<S: RootStack> St<'_, '_, '_, S> {
    fn load(&mut self, function: u32, base: usize, pc: usize) {
        let f = &self.cx.loaded.program.functions[function as usize];
        self.function = function;
        self.base = base;
        self.pc = pc;
        self.code = &f.code;
        self.consts = &f.consts;
        self.w = window(self.cx.regs, base);
    }

    fn fail(&mut self, error: VmError) -> Flow {
        self.outcome = Err(error);
        Flow::Stop
    }
}

pub(crate) fn run<S: RootStack>(
    cx: &mut Cx<'_, '_, S>,
    function: u32,
    base: usize,
    stop: usize,
    gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let f = &cx.loaded.program.functions[function as usize];
    let (code, consts, w) = (&f.code[..], &f.consts[..], window(cx.regs, base));
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
    let handlers = &Table::<S>::HANDLERS;
    loop {
        let i = st.code[st.pc];
        st.pc += 1;
        st.cx.core.stats.instruction(i.op);
        if handlers[usize::from(i.op)](&mut st, i) == Flow::Stop {
            return st.outcome;
        }
    }
}

struct Table<S>(PhantomData<S>);

impl<S: RootStack> Table<S> {
    const HANDLERS: [Handler<S>; 256] = {
        let mut t = [illegal::<S> as Handler<S>; 256];
        t[op::LOAD_INT as usize] = load_int::<S>;
        t[op::LOAD_CONST as usize] = load_const::<S>;
        t[op::LOAD_MISC as usize] = load_misc::<S>;
        t[op::MOVE as usize] = mov::<S>;
        t[op::ADD as usize] = add::<S>;
        t[op::SUB as usize] = sub::<S>;
        t[op::MUL as usize] = mul::<S>;
        t[op::DIV as usize] = div::<S>;
        t[op::MOD as usize] = rem::<S>;
        t[op::ADD_IMM as usize] = add_imm::<S>;
        t[op::LT as usize] = lt::<S>;
        t[op::LE as usize] = le::<S>;
        t[op::EQ as usize] = eq::<S>;
        t[op::JUMP as usize] = jump_always::<S>;
        t[op::JUMP_IF as usize] = jump_if::<S>;
        t[op::JUMP_IF_NOT as usize] = jump_if_not::<S>;
        t[op::NEW_OBJECT as usize] = new_object::<S>;
        t[op::GET_SLOT as usize] = get_slot::<S>;
        t[op::SET_SLOT as usize] = set_slot::<S>;
        t[op::NEW_ARRAY as usize] = new_array::<S>;
        t[op::GET_INDEX as usize] = get_index::<S>;
        t[op::SET_INDEX as usize] = set_index::<S>;
        t[op::LENGTH as usize] = length::<S>;
        t[op::CLOSURE as usize] = closure::<S>;
        t[op::GET_ENV as usize] = get_env::<S>;
        t[op::SET_ENV as usize] = set_env::<S>;
        t[op::CALL as usize] = call::<S>;
        t[op::RETURN as usize] = ret::<S>;
        t
    };
}

fn illegal<S: RootStack>(_: &mut St<'_, '_, '_, S>, _: Instr) -> Flow {
    unreachable!("verified programs have no unknown opcodes")
}

/// Handlers for the operations without control flow.
macro_rules! handlers {
    ($($name:ident => $method:ident),+ $(,)?) => {
        $(
            fn $name<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
                match st.cx.$method(st.w, st.consts, st.base, i, st.gc.reborrow()) {
                    Ok(()) => Flow::Next,
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

fn jump_always<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
    st.pc = jump(st.pc, i.sbx());
    Flow::Next
}

fn jump_if<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
    if truthy(reg(st.w, i.a).get()) {
        st.pc = jump(st.pc, i.sbx());
    }
    Flow::Next
}

fn jump_if_not<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
    if !truthy(reg(st.w, i.a).get()) {
        st.pc = jump(st.pc, i.sbx());
    }
    Flow::Next
}

fn call<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
    let callee = reg(st.w, i.a).get();
    if let Some(closure) = callee.to_handle::<Closure>() {
        match st.cx.enter(closure, st.function, st.pc, st.base, i) {
            Ok((function, base)) => {
                st.load(function, base, 0);
                Flow::Next
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
                Flow::Next
            }
            Err(e) => st.fail(e),
        }
    }
}

fn ret<S: RootStack>(st: &mut St<'_, '_, '_, S>, i: Instr) -> Flow {
    let v = reg(st.w, i.a).get();
    match st.cx.leave(v, st.stop) {
        None => {
            st.outcome = Ok(v);
            Flow::Stop
        }
        Some(caller) => {
            st.load(caller.function, caller.base as usize, caller.pc as usize);
            Flow::Next
        }
    }
}
