//! Dispatch by one `match` on the opcode. The frame's state lives in locals.

use s3_gc::{GcScope, RootStack, Value};

use crate::bytecode::{Instr, op};
use crate::heap::Closure;
use crate::machine::{Cx, VmError, jump, reg, truthy, window};

/// Run the frame of `function` at `base` until the run that started at frame
/// depth `stop` returns.
pub(crate) fn run<'a, S: RootStack>(
    cx: &mut Cx<'a, '_, S>,
    mut function: u32,
    mut base: usize,
    stop: usize,
    mut gc: GcScope<'_>,
) -> Result<Value, VmError> {
    let program = &cx.loaded.program;
    let regs = cx.regs;
    let mut f = &program.functions[function as usize];
    let mut code: &'a [Instr] = &f.code;
    let mut consts: &'a [Value] = &f.consts;
    let mut w = window(regs, base);
    let mut pc = 0;
    loop {
        let i = code[pc];
        pc += 1;
        cx.core.stats.instruction(i.op);
        match i.op {
            op::LOAD_INT => cx.op_load_int(w, consts, base, i, gc.reborrow())?,
            op::LOAD_CONST => cx.op_load_const(w, consts, base, i, gc.reborrow())?,
            op::LOAD_MISC => cx.op_load_misc(w, consts, base, i, gc.reborrow())?,
            op::MOVE => cx.op_move(w, consts, base, i, gc.reborrow())?,
            op::ADD => cx.op_add(w, consts, base, i, gc.reborrow())?,
            op::SUB => cx.op_sub(w, consts, base, i, gc.reborrow())?,
            op::MUL => cx.op_mul(w, consts, base, i, gc.reborrow())?,
            op::DIV => cx.op_div(w, consts, base, i, gc.reborrow())?,
            op::MOD => cx.op_mod(w, consts, base, i, gc.reborrow())?,
            op::ADD_IMM => cx.op_add_imm(w, consts, base, i, gc.reborrow())?,
            op::LT => cx.op_lt(w, consts, base, i, gc.reborrow())?,
            op::LE => cx.op_le(w, consts, base, i, gc.reborrow())?,
            op::EQ => cx.op_eq(w, consts, base, i, gc.reborrow())?,
            op::JUMP => pc = jump(pc, i.sbx()),
            op::JUMP_IF => {
                if truthy(reg(w, i.a).get()) {
                    pc = jump(pc, i.sbx());
                }
            }
            op::JUMP_IF_NOT => {
                if !truthy(reg(w, i.a).get()) {
                    pc = jump(pc, i.sbx());
                }
            }
            op::NEW_OBJECT => cx.op_new_object(w, consts, base, i, gc.reborrow())?,
            op::GET_SLOT => cx.op_get_slot(w, consts, base, i, gc.reborrow())?,
            op::SET_SLOT => cx.op_set_slot(w, consts, base, i, gc.reborrow())?,
            op::NEW_ARRAY => cx.op_new_array(w, consts, base, i, gc.reborrow())?,
            op::GET_INDEX => cx.op_get_index(w, consts, base, i, gc.reborrow())?,
            op::SET_INDEX => cx.op_set_index(w, consts, base, i, gc.reborrow())?,
            op::LENGTH => cx.op_length(w, consts, base, i, gc.reborrow())?,
            op::CLOSURE => cx.op_closure(w, consts, base, i, gc.reborrow())?,
            op::GET_ENV => cx.op_get_env(w, consts, base, i, gc.reborrow())?,
            op::SET_ENV => cx.op_set_env(w, consts, base, i, gc.reborrow())?,
            op::CALL => {
                let callee = reg(w, i.a).get();
                if let Some(closure) = callee.to_handle::<Closure>() {
                    (function, base) = cx.enter(closure, function, pc, base, i)?;
                    f = &program.functions[function as usize];
                    code = &f.code;
                    consts = &f.consts;
                    w = window(regs, base);
                    pc = 0;
                } else {
                    let v = cx.call_native(callee, w, base, i, gc.reborrow())?;
                    reg(w, i.a).set(v);
                }
            }
            op::RETURN => {
                let v = reg(w, i.a).get();
                let Some(caller) = cx.leave(v, stop) else {
                    return Ok(v);
                };
                function = caller.function;
                base = caller.base as usize;
                pc = caller.pc as usize;
                f = &program.functions[function as usize];
                code = &f.code;
                consts = &f.consts;
                w = window(regs, base);
            }
            _ => unreachable!("verified programs have no unknown opcodes"),
        }
    }
}
