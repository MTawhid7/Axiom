//! A small assembler for writing bytecode by hand: registers, labels, and one
//! method per instruction.

use s3_gc::Value;

use crate::builtins::Builtin;
use crate::bytecode::{Function, Instr, Program, op};

/// A register of the function being built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reg(pub u8);

/// A jump target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(usize);

/// A function of the program being built, usable before it is defined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuncId(pub u8);

/// Builds a [`Program`]. Declare functions first, so they can refer to each
/// other, then define them.
#[derive(Debug, Default)]
pub struct ProgramBuilder {
    functions: Vec<Option<Function>>,
    names: Vec<String>,
}

impl ProgramBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reserve a function. The first one declared is the entry point.
    ///
    /// # Panics
    /// Beyond 256 functions.
    pub fn declare(&mut self, name: &str) -> FuncId {
        let id = u8::try_from(self.functions.len()).expect("at most 256 functions");
        self.functions.push(None);
        self.names.push(name.to_owned());
        FuncId(id)
    }

    /// Start the body of a declared function.
    #[must_use]
    pub fn body(&self, id: FuncId, params: u8) -> FnBuilder {
        FnBuilder::new(&self.names[usize::from(id.0)], params, id)
    }

    /// Finish a function's body.
    ///
    /// # Panics
    /// If a label is used but never bound, or the function was defined already.
    pub fn define(&mut self, f: FnBuilder) {
        let id = usize::from(f.id.0);
        assert!(self.functions[id].is_none(), "{} defined twice", f.name);
        self.functions[id] = Some(f.finish());
    }

    /// The verified program.
    ///
    /// # Errors
    /// If a function was declared but not defined, or fails verification.
    pub fn build(self) -> Result<Program, String> {
        let functions = self
            .functions
            .into_iter()
            .zip(&self.names)
            .map(|(f, name)| f.ok_or_else(|| format!("{name} was declared but not defined")))
            .collect::<Result<Vec<_>, _>>()?;
        let program = Program { functions };
        program.verify()?;
        Ok(program)
    }
}

/// Builds one function's code. Registers are allocated in order with
/// [`reg`](Self::reg), starting after the parameters.
#[derive(Debug)]
pub struct FnBuilder {
    id: FuncId,
    name: String,
    params: u8,
    next_reg: u16,
    code: Vec<Instr>,
    consts: Vec<Value>,
    labels: Vec<Option<usize>>,
    fixups: Vec<(usize, Label)>,
}

impl FnBuilder {
    fn new(name: &str, params: u8, id: FuncId) -> Self {
        Self {
            id,
            name: name.to_owned(),
            params,
            next_reg: u16::from(params),
            code: Vec::new(),
            consts: Vec::new(),
            labels: Vec::new(),
            fixups: Vec::new(),
        }
    }

    /// Parameter `i`.
    ///
    /// # Panics
    /// If the function has no such parameter.
    #[must_use]
    pub fn param(&self, i: u8) -> Reg {
        assert!(i < self.params, "{}: no parameter {i}", self.name);
        Reg(i)
    }

    /// The next free register.
    ///
    /// # Panics
    /// Beyond 256 registers.
    pub fn reg(&mut self) -> Reg {
        let r = u8::try_from(self.next_reg).expect("at most 256 registers");
        self.next_reg += 1;
        Reg(r)
    }

    /// `n` consecutive registers.
    pub fn regs<const N: usize>(&mut self) -> [Reg; N] {
        core::array::from_fn(|_| self.reg())
    }

    pub fn label(&mut self) -> Label {
        self.labels.push(None);
        Label(self.labels.len() - 1)
    }

    /// Place `label` at the next instruction.
    ///
    /// # Panics
    /// If the label was placed already.
    pub fn bind(&mut self, label: Label) {
        assert!(self.labels[label.0].is_none(), "label bound twice");
        self.labels[label.0] = Some(self.code.len());
    }

    fn emit(&mut self, i: Instr) {
        self.code.push(i);
    }

    fn jump_to(&mut self, opcode: u8, a: u8, target: Label) {
        self.fixups.push((self.code.len(), target));
        self.emit(Instr::new(opcode, a, 0, 0));
    }

    fn constant(&mut self, value: Value) -> u16 {
        let index = self
            .consts
            .iter()
            .position(|&k| k == value)
            .unwrap_or_else(|| {
                self.consts.push(value);
                self.consts.len() - 1
            });
        u16::try_from(index).expect("at most 65,536 constants")
    }

    // ----- Loads and moves -----

    /// Load an integer.
    pub fn int(&mut self, dst: Reg, n: i32) {
        match i16::try_from(n) {
            Ok(small) => self.emit(Instr::with_bx(op::LOAD_INT, dst.0, small.cast_unsigned())),
            Err(_) => self.load(dst, Value::int(n)),
        }
    }

    /// Load a double.
    pub fn number(&mut self, dst: Reg, x: f64) {
        self.load(dst, Value::number(x));
    }

    /// Load a builtin function.
    pub fn builtin(&mut self, dst: Reg, builtin: Builtin) {
        self.load(dst, builtin.value());
    }

    fn load(&mut self, dst: Reg, value: Value) {
        let k = self.constant(value);
        self.emit(Instr::with_bx(op::LOAD_CONST, dst.0, k));
    }

    pub fn undefined(&mut self, dst: Reg) {
        self.emit(Instr::new(op::LOAD_MISC, dst.0, 0, 0));
    }

    pub fn null(&mut self, dst: Reg) {
        self.emit(Instr::new(op::LOAD_MISC, dst.0, 1, 0));
    }

    pub fn boolean(&mut self, dst: Reg, b: bool) {
        self.emit(Instr::new(op::LOAD_MISC, dst.0, 2 + u8::from(b), 0));
    }

    pub fn mov(&mut self, dst: Reg, src: Reg) {
        self.emit(Instr::new(op::MOVE, dst.0, src.0, 0));
    }

    // ----- Arithmetic and comparison -----

    pub fn add(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::ADD, dst.0, x.0, y.0));
    }

    pub fn sub(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::SUB, dst.0, x.0, y.0));
    }

    pub fn mul(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::MUL, dst.0, x.0, y.0));
    }

    pub fn div(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::DIV, dst.0, x.0, y.0));
    }

    pub fn rem(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::MOD, dst.0, x.0, y.0));
    }

    pub fn add_imm(&mut self, dst: Reg, x: Reg, n: i8) {
        self.emit(Instr::new(op::ADD_IMM, dst.0, x.0, n.cast_unsigned()));
    }

    pub fn lt(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::LT, dst.0, x.0, y.0));
    }

    pub fn le(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::LE, dst.0, x.0, y.0));
    }

    pub fn eq(&mut self, dst: Reg, x: Reg, y: Reg) {
        self.emit(Instr::new(op::EQ, dst.0, x.0, y.0));
    }

    // ----- Control -----

    pub fn jump(&mut self, target: Label) {
        self.jump_to(op::JUMP, 0, target);
    }

    pub fn jump_if(&mut self, cond: Reg, target: Label) {
        self.jump_to(op::JUMP_IF, cond.0, target);
    }

    pub fn jump_if_not(&mut self, cond: Reg, target: Label) {
        self.jump_to(op::JUMP_IF_NOT, cond.0, target);
    }

    /// Call `f` with the `args` registers that follow it; the result replaces
    /// `f`. Every register above `f` is clobbered.
    ///
    /// # Panics
    /// If the arguments are not in the registers right after `f`.
    pub fn call(&mut self, f: Reg, args: &[Reg]) {
        for (k, arg) in args.iter().enumerate() {
            assert_eq!(
                usize::from(arg.0),
                usize::from(f.0) + 1 + k,
                "{}: arguments must follow the callee",
                self.name
            );
        }
        let n = u8::try_from(args.len()).expect("few arguments");
        self.emit(Instr::new(op::CALL, f.0, n, 0));
    }

    pub fn ret(&mut self, value: Reg) {
        self.emit(Instr::new(op::RETURN, value.0, 0, 0));
    }

    // ----- Objects, arrays, closures -----

    pub fn new_object(&mut self, dst: Reg) {
        self.emit(Instr::new(op::NEW_OBJECT, dst.0, 0, 0));
    }

    pub fn get_slot(&mut self, dst: Reg, object: Reg, slot: u8) {
        self.emit(Instr::new(op::GET_SLOT, dst.0, object.0, slot));
    }

    pub fn set_slot(&mut self, object: Reg, slot: u8, value: Reg) {
        self.emit(Instr::new(op::SET_SLOT, object.0, slot, value.0));
    }

    pub fn new_array(&mut self, dst: Reg, len: Reg) {
        self.emit(Instr::new(op::NEW_ARRAY, dst.0, len.0, 0));
    }

    pub fn get_index(&mut self, dst: Reg, array: Reg, index: Reg) {
        self.emit(Instr::new(op::GET_INDEX, dst.0, array.0, index.0));
    }

    pub fn set_index(&mut self, array: Reg, index: Reg, value: Reg) {
        self.emit(Instr::new(op::SET_INDEX, array.0, index.0, value.0));
    }

    pub fn length(&mut self, dst: Reg, array: Reg) {
        self.emit(Instr::new(op::LENGTH, dst.0, array.0, 0));
    }

    pub fn closure(&mut self, dst: Reg, function: FuncId, env: Reg) {
        self.emit(Instr::new(op::CLOSURE, dst.0, function.0, env.0));
    }

    pub fn get_env(&mut self, dst: Reg, slot: u8) {
        self.emit(Instr::new(op::GET_ENV, dst.0, slot, 0));
    }

    pub fn set_env(&mut self, slot: u8, value: Reg) {
        self.emit(Instr::new(op::SET_ENV, slot, value.0, 0));
    }

    fn finish(mut self) -> Function {
        for &(at, label) in &self.fixups {
            let target = self.labels[label.0].expect("a label was used but never bound");
            let offset = i16::try_from(target.cast_signed() - (at.cast_signed() + 1))
                .expect("jump within ±32k instructions");
            let i = self.code[at];
            self.code[at] = Instr::with_bx(i.op, i.a, offset.cast_unsigned());
        }
        Function {
            name: self.name,
            params: self.params,
            code: self.code,
            consts: self.consts,
        }
    }
}
