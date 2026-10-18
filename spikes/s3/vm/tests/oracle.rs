//! The oracle: every program must give the reference evaluator's result on
//! every dispatch technique and every root stack, at default pacing and when
//! collecting at every allocation. Debug builds also check every handle's
//! collection epoch, so a value held across a collection without a root
//! fails here even if it happens to read the right object.
#![forbid(unsafe_code)]
#![expect(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "assembly style: short register names, as in the bytecode docs"
)]

use s3_gc::{
    CellStack, DEFAULT_CAPACITY, Pacing, RawStack, RefCellStack, RootStack, SliceStack, Value,
    slot_buffer,
};
use s3_vm::workloads::{self, Size};
use s3_vm::{Builtin, FnBuilder, Loaded, Program, ProgramBuilder, Reg, Technique, Vm, VmError};

/// Run `f` on a thread with a large stack: the reference evaluator recurses
/// once per call, and debug-build frames are big.
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("the test thread panicked")
}

fn run_on<S: RootStack>(
    stack: &S,
    program: &Program,
    args: &[Value],
    technique: Technique,
    pacing: Pacing,
) -> Result<u64, VmError> {
    let loaded = Loaded::<S>::new(program.clone()).expect("verified");
    let mut vm = Vm::new(stack, pacing);
    let result = vm.run(&loaded, technique, args).map(|v| vm.digest(v));
    assert_eq!(stack.height(), 0, "every root frame was popped");
    result
}

/// Every configuration's result, checked against the reference's.
fn check(program: &Program, args: &[Value]) -> Result<u64, VmError> {
    let expected = s3_vm::reference::run(program, args).result;
    for &technique in Technique::ALL {
        for pacing in [Pacing::default(), Pacing::STRESS] {
            let what = format!("{technique}, {pacing:?}");
            assert_eq!(
                run_on(&RefCellStack::default(), program, args, technique, pacing),
                expected,
                "RefCellStack, {what}"
            );
            assert_eq!(
                run_on(&CellStack::default(), program, args, technique, pacing),
                expected,
                "CellStack, {what}"
            );
            assert_eq!(
                run_on(&RawStack::default(), program, args, technique, pacing),
                expected,
                "RawStack, {what}"
            );
            let mut buffer = slot_buffer(DEFAULT_CAPACITY);
            assert_eq!(
                run_on(
                    &SliceStack::new(&mut buffer),
                    program,
                    args,
                    technique,
                    pacing
                ),
                expected,
                "SliceStack, {what}"
            );
        }
    }
    expected
}

#[test]
fn every_workload_matches_its_native_model_everywhere() {
    big_stack(|| {
        for w in workloads::all(Size::Test) {
            assert_eq!(
                s3_vm::reference::run(&w.program, &w.args).result,
                Ok(w.expected),
                "{}: the reference against the native model",
                w.name
            );
            assert_eq!(check(&w.program, &w.args), Ok(w.expected), "{}", w.name);
        }
    });
}

// ----- Edge cases: small programs, one concern each -----

/// A one-function program: `body` gets the builder and returns the register
/// to return.
fn main_only(params: u8, body: impl FnOnce(&mut FnBuilder) -> Reg) -> Program {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let mut f = b.body(main, params);
    let result = body(&mut f);
    f.ret(result);
    b.define(f);
    b.build().expect("valid")
}

/// Evaluate `pairs` of operands with a binary operation; returns an array of
/// the results.
fn binary_table(op: fn(&mut FnBuilder, Reg, Reg, Reg), operands: &[(Value, Value)]) -> Program {
    main_only(0, |f| {
        let [out, x, y, r, i] = f.regs();
        f.int(r, i32::try_from(operands.len()).expect("few"));
        f.new_array(out, r);
        for (k, &(a, b)) in operands.iter().enumerate() {
            load(f, x, a);
            load(f, y, b);
            op(f, r, x, y);
            f.int(i, i32::try_from(k).expect("few"));
            f.set_index(out, i, r);
        }
        out
    })
}

fn load(f: &mut FnBuilder, dst: Reg, v: Value) {
    if let Some(n) = v.as_int() {
        f.int(dst, n);
    } else if let Some(x) = v.as_f64() {
        f.number(dst, x);
    } else if v == Value::UNDEFINED {
        f.undefined(dst);
    } else if v == Value::NULL {
        f.null(dst);
    } else if let Some(b) = v.as_bool() {
        f.boolean(dst, b);
    } else {
        unreachable!("primitive operands only")
    }
}

fn numbers() -> Vec<Value> {
    vec![
        Value::int(0),
        Value::int(-1),
        Value::int(7),
        Value::int(i32::MAX),
        Value::int(i32::MIN),
        Value::number(-0.0),
        Value::number(2.5),
        Value::number(f64::NAN),
        Value::number(f64::INFINITY),
        Value::UNDEFINED,
        Value::NULL,
        Value::TRUE,
    ]
}

#[test]
fn arithmetic_matches_doubles_for_every_operand_pair() {
    big_stack(|| {
        let values = numbers();
        let pairs: Vec<_> = values
            .iter()
            .flat_map(|&a| values.iter().map(move |&b| (a, b)))
            .collect();
        for op in [
            FnBuilder::add,
            FnBuilder::sub,
            FnBuilder::mul,
            FnBuilder::div,
            FnBuilder::rem,
            FnBuilder::lt,
            FnBuilder::le,
            FnBuilder::eq,
        ] {
            check(&binary_table(op, &pairs), &[]).expect("arithmetic never fails");
        }
    });
}

#[test]
fn negative_zero_survives_the_integer_fast_paths() {
    let program = binary_table(
        FnBuilder::mul,
        &[
            (Value::int(0), Value::int(-5)),
            (Value::int(-3), Value::int(0)),
        ],
    );
    let expected = s3_vm::digest::Digest::of_numbers(&[-0.0, -0.0]);
    assert_eq!(big_stack(move || check(&program, &[])), Ok(expected));
    let program = binary_table(
        FnBuilder::rem,
        &[
            (Value::int(-4), Value::int(2)),
            (Value::int(i32::MIN), Value::int(-1)),
        ],
    );
    let expected = s3_vm::digest::Digest::of_numbers(&[-0.0, -0.0]);
    assert_eq!(big_stack(move || check(&program, &[])), Ok(expected));
    let program = binary_table(
        FnBuilder::div,
        &[
            (Value::int(0), Value::int(-5)),
            (Value::int(i32::MIN), Value::int(-1)),
        ],
    );
    let expected = s3_vm::digest::Digest::of_numbers(&[-0.0, 2_147_483_648.0]);
    assert_eq!(big_stack(move || check(&program, &[])), Ok(expected));
}

#[test]
fn truthiness_drives_branches() {
    big_stack(|| {
        for v in numbers() {
            let program = main_only(0, |f| {
                let [x, out] = f.regs();
                let (yes, end) = (f.label(), f.label());
                load(f, x, v);
                f.jump_if(x, yes);
                f.int(out, 0);
                f.jump(end);
                f.bind(yes);
                f.int(out, 1);
                f.bind(end);
                out
            });
            check(&program, &[]).expect("branches never fail");
        }
    });
}

#[test]
fn arrays_read_undefined_past_the_end_and_fill_gaps() {
    let program = main_only(0, |f| {
        let [a, i, v, n] = f.regs();
        f.int(n, 2);
        f.new_array(a, n);
        f.int(i, 5);
        f.int(v, 9);
        f.set_index(a, i, v); // fills 2, 3, 4 with undefined
        f.int(i, 7);
        f.get_index(v, a, i); // past the end: undefined
        f.int(i, 0);
        f.set_index(a, i, v);
        f.length(n, a);
        f.int(i, 1);
        f.set_index(a, i, n);
        a
    });
    assert!(big_stack(move || check(&program, &[])).is_ok());
}

#[test]
fn errors_are_the_same_everywhere() {
    let cases: Vec<(Program, VmError)> = vec![
        (
            main_only(0, |f| {
                let [x] = f.regs();
                f.int(x, 1);
                f.get_slot(x, x, 0);
                x
            }),
            VmError::NotObject,
        ),
        (
            main_only(0, |f| {
                let [x] = f.regs();
                f.new_object(x);
                f.length(x, x);
                x
            }),
            VmError::NotArray,
        ),
        (
            main_only(0, |f| {
                let [x] = f.regs();
                f.int(x, 3);
                f.call(x, &[]);
                x
            }),
            VmError::NotCallable,
        ),
        (
            main_only(0, |f| {
                let [a, i] = f.regs();
                f.int(i, 1);
                f.new_array(a, i);
                f.int(i, -1);
                f.get_index(i, a, i);
                i
            }),
            VmError::BadIndex,
        ),
        (
            main_only(0, |f| {
                let [x] = f.regs();
                f.get_env(x, 0); // the entry closure has no environment
                x
            }),
            VmError::NotObject,
        ),
    ];
    for (program, error) in cases {
        assert_eq!(big_stack(move || check(&program, &[])), Err(error));
    }
}

/// `again(f) = f(f)`, forever, allocating nothing.
fn infinite_recursion() -> Program {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let again = b.declare("again");
    let mut f = b.body(again, 1);
    let this = f.param(0);
    let [callee, arg] = f.regs();
    f.mov(callee, this);
    f.mov(arg, this);
    f.call(callee, &[arg]);
    f.ret(callee);
    b.define(f);
    let mut f = b.body(main, 0);
    let [env, callee, arg] = f.regs();
    f.undefined(env);
    f.closure(callee, again, env);
    f.mov(arg, callee);
    f.call(callee, &[arg]);
    f.ret(callee);
    b.define(f);
    b.build().expect("valid")
}

#[test]
fn unbounded_recursion_overflows_cleanly() {
    let program = infinite_recursion();
    assert_eq!(
        big_stack(move || check(&program, &[])),
        Err(VmError::StackOverflow)
    );
}

/// `n` closures, each adding its index to a counter in their shared
/// environment, called in order; then a recursive countdown of depth `n`.
fn closures_and_recursion(n: i32) -> Program {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let bump = b.declare("bump");
    let down = b.declare("down");
    let mut f = b.body(bump, 1);
    let (k, acc) = (f.param(0), f.reg());
    f.get_env(acc, 0);
    f.add(acc, acc, k);
    f.set_env(0, acc);
    f.ret(acc);
    b.define(f);
    let mut f = b.body(down, 1);
    let k = f.param(0);
    let [t, env] = f.regs();
    let [callee, arg] = f.regs();
    let base = f.label();
    f.int(t, 0);
    f.le(t, k, t);
    f.jump_if(t, base);
    f.undefined(env);
    f.closure(callee, down, env);
    f.add_imm(arg, k, -1);
    f.call(callee, &[arg]);
    f.add_imm(callee, callee, 1);
    f.ret(callee);
    f.bind(base);
    f.ret(k);
    b.define(f);
    let mut f = b.body(main, 0);
    let [env, fns, i, n_reg, t, out] = f.regs();
    let [callee, arg] = f.regs();
    f.new_object(env);
    f.int(t, 0);
    f.set_slot(env, 0, t);
    f.int(n_reg, n);
    f.new_array(fns, n_reg);
    let (fill, filled, call_all, called) = (f.label(), f.label(), f.label(), f.label());
    f.int(i, 0);
    f.bind(fill);
    f.lt(t, i, n_reg);
    f.jump_if_not(t, filled);
    f.closure(t, bump, env);
    f.set_index(fns, i, t);
    f.add_imm(i, i, 1);
    f.jump(fill);
    f.bind(filled);
    f.int(i, 0);
    f.bind(call_all);
    f.lt(t, i, n_reg);
    f.jump_if_not(t, called);
    f.get_index(callee, fns, i);
    f.mov(arg, i);
    f.call(callee, &[arg]);
    f.add_imm(i, i, 1);
    f.jump(call_all);
    f.bind(called);
    f.int(t, 2);
    f.new_array(out, t);
    f.get_slot(t, env, 0);
    f.int(i, 0);
    f.set_index(out, i, t);
    f.undefined(t);
    f.closure(callee, down, t);
    f.mov(arg, n_reg);
    f.call(callee, &[arg]);
    f.int(i, 1);
    f.set_index(out, i, callee);
    f.ret(out);
    b.define(f);
    b.build().expect("valid")
}

#[test]
fn closures_share_environments_and_recursion_returns_through_every_frame() {
    let program = closures_and_recursion(300);
    let expected = s3_vm::digest::Digest::of_numbers(&[44_850.0, 300.0]);
    assert_eq!(big_stack(move || check(&program, &[])), Ok(expected));
}

/// Builtins with callbacks that misbehave: grow and rewrite the array being
/// visited, count their calls, return NaN or objects with hooks, allocate,
/// and nest builtins.
#[expect(clippy::too_many_lines, reason = "an assembly listing")]
fn misbehaving_callbacks() -> Program {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let grow = b.declare("grow");
    let count_cmp = b.declare("countCmp");
    let nan_cmp = b.declare("nanCmp");
    let boxed_cmp = b.declare("boxedCmp");
    let value_of = b.declare("valueOf");
    let alloc_map = b.declare("allocMap");
    let nested = b.declare("nested");
    let inner = b.declare("inner");

    // grow(x, i): env.0[20] = x, past the length forEach read; env.0[i + 1] = i,
    // which forEach reads next
    let mut f = b.body(grow, 2);
    let (x, i) = (f.param(0), f.param(1));
    let [arr, t] = f.regs();
    f.get_env(arr, 0);
    f.int(t, 20);
    f.set_index(arr, t, x);
    f.add_imm(t, i, 1);
    f.set_index(arr, t, i);
    f.ret(x);
    b.define(f);

    // countCmp(a, b): env.1 += 1; return a - b
    let mut f = b.body(count_cmp, 2);
    let (a, c, n) = (f.param(0), f.param(1), f.reg());
    f.get_env(n, 1);
    f.add_imm(n, n, 1);
    f.set_env(1, n);
    f.sub(a, a, c);
    f.ret(a);
    b.define(f);

    // nanCmp(a, b) = NaN: every element keeps its place
    let mut f = b.body(nan_cmp, 2);
    let r = f.reg();
    f.number(r, f64::NAN);
    f.ret(r);
    b.define(f);

    // boxedCmp(a, b) = an object whose hook returns a - b
    let mut f = b.body(boxed_cmp, 2);
    let (a, c) = (f.param(0), f.param(1));
    let [o, hook, env] = f.regs();
    f.sub(a, a, c);
    f.new_object(env);
    f.set_slot(env, 0, a);
    f.new_object(o);
    f.closure(hook, value_of, env);
    f.set_slot(o, 3, hook);
    f.ret(o);
    b.define(f);

    // valueOf(self) = env.0, allocating garbage first
    let mut f = b.body(value_of, 1);
    let [g, v] = f.regs();
    f.new_object(g);
    f.new_object(g);
    f.get_env(v, 0);
    f.ret(v);
    b.define(f);

    // allocMap(x, i) = [x, i], allocating
    let mut f = b.body(alloc_map, 2);
    let (x, i) = (f.param(0), f.param(1));
    let [out, t] = f.regs();
    f.int(t, 2);
    f.new_array(out, t);
    f.int(t, 0);
    f.set_index(out, t, x);
    f.int(t, 1);
    f.set_index(out, t, i);
    f.ret(out);
    b.define(f);

    // inner(x, i): env.2 += x
    let mut f = b.body(inner, 2);
    let (x, acc) = (f.param(0), f.reg());
    f.get_env(acc, 2);
    f.add(acc, acc, x);
    f.set_env(2, acc);
    f.ret(acc);
    b.define(f);

    // nested(row, i): forEach(row, inner) — a builtin inside a builtin's callback
    let mut f = b.body(nested, 2);
    let row = f.param(0);
    let [env, callee, arg0, arg1] = f.regs();
    f.undefined(env);
    f.get_env(arg1, 3); // the shared environment, kept in itself
    f.mov(env, arg1);
    f.builtin(callee, Builtin::ForEach);
    f.mov(arg0, row);
    f.closure(arg1, inner, env);
    f.call(callee, &[arg0, arg1]);
    f.ret(callee);
    b.define(f);

    let mut f = b.body(main, 0);
    let [env, arr, i, t, n, out, mapped, k] = f.regs();
    let [callee, arg0, arg1] = f.regs();
    f.new_object(env);
    f.set_slot(env, 3, env); // env.3 = env: a cycle
    f.int(n, 9);
    f.new_array(arr, n);
    let (fill, filled) = (f.label(), f.label());
    f.int(i, 0);
    f.bind(fill);
    f.lt(t, i, n);
    f.jump_if_not(t, filled);
    f.int(t, 3);
    f.mul(t, i, t);
    f.int(k, 7);
    f.rem(t, t, k);
    f.set_index(arr, i, t);
    f.add_imm(i, i, 1);
    f.jump(fill);
    f.bind(filled);
    f.int(t, 0);
    f.set_slot(env, 1, t);
    f.set_slot(env, 2, t);
    let call2 = |f: &mut FnBuilder, builtin: Builtin, array: Reg, function| {
        f.builtin(callee, builtin);
        f.mov(arg0, array);
        f.closure(arg1, function, env);
        f.call(callee, &[arg0, arg1]);
    };
    // A side-effecting comparator: the number of calls is part of the result.
    call2(&mut f, Builtin::SortBy, arr, count_cmp);
    call2(&mut f, Builtin::SortBy, arr, nan_cmp);
    call2(&mut f, Builtin::SortBy, arr, boxed_cmp);
    call2(&mut f, Builtin::Map, arr, alloc_map);
    f.mov(mapped, callee);
    call2(&mut f, Builtin::ForEach, mapped, nested);
    // forEach over an array its callback grows and rewrites.
    f.set_slot(env, 0, arr);
    call2(&mut f, Builtin::ForEach, arr, grow);
    f.int(t, 4);
    f.new_array(out, t);
    f.int(i, 0);
    f.set_index(out, i, arr);
    f.int(i, 1);
    f.set_index(out, i, mapped);
    f.int(i, 2);
    f.set_index(out, i, env);
    f.ret(out);
    b.define(f);
    b.build().expect("valid")
}

#[test]
fn misbehaving_callbacks_behave_the_same_everywhere() {
    let program = misbehaving_callbacks();
    assert!(big_stack(move || check(&program, &[])).is_ok());
}

#[test]
fn a_hook_that_returns_an_object_is_an_error() {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let hook = b.declare("hook");
    let mut f = b.body(hook, 1);
    let this = f.param(0);
    f.ret(this);
    b.define(f);
    let mut f = b.body(main, 0);
    let [o, h, env] = f.regs();
    f.new_object(o);
    f.undefined(env);
    f.closure(h, hook, env);
    f.set_slot(o, 3, h);
    f.add(h, o, o);
    f.ret(h);
    b.define(f);
    let program = b.build().expect("valid");
    assert_eq!(
        big_stack(move || check(&program, &[])),
        Err(VmError::NotPrimitive)
    );
}

#[test]
fn missing_arguments_are_undefined_and_extra_ones_are_ignored() {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let pair = b.declare("pair");
    let mut f = b.body(pair, 2);
    let (x, y) = (f.param(0), f.param(1));
    let [out, i] = f.regs();
    f.int(i, 2);
    f.new_array(out, i);
    f.int(i, 0);
    f.set_index(out, i, x);
    f.int(i, 1);
    f.set_index(out, i, y);
    f.ret(out);
    b.define(f);
    let mut f = b.body(main, 1);
    let p = f.param(0);
    let [env, out, i] = f.regs();
    let [callee, a0, a1, a2] = f.regs();
    f.int(i, 2);
    f.new_array(out, i);
    f.undefined(env);
    f.closure(callee, pair, env);
    f.mov(a0, p);
    f.call(callee, &[a0]);
    f.int(i, 0);
    f.set_index(out, i, callee);
    f.closure(callee, pair, env);
    f.int(a0, 1);
    f.int(a1, 2);
    f.int(a2, 3);
    f.call(callee, &[a0, a1, a2]);
    f.int(i, 1);
    f.set_index(out, i, callee);
    f.ret(out);
    b.define(f);
    let program = b.build().expect("valid");
    assert!(big_stack(move || check(&program, &[Value::int(5)])).is_ok());
}

#[test]
#[should_panic(expected = "entry arguments are primitives")]
fn entry_arguments_must_be_primitives() {
    let stack = CellStack::default();
    let mut vm = Vm::new(&stack, Pacing::default());
    let program = main_only(1, |f| f.param(0));
    let loaded = Loaded::<CellStack>::new(program).expect("valid");
    // A heap reference from another run's heap: never valid as an argument.
    let w = workloads::w4_tree(3, 1);
    let other = Loaded::<CellStack>::new(w.program).expect("valid");
    let leaked = vm.run(&other, Technique::Match, &w.args).expect("runs");
    let _ = vm.run(&loaded, Technique::Match, &[leaked]);
}

/// S3-Q2's first criterion, checked directly: executing bytecode performs no
/// root operations; builtins do.
#[test]
#[cfg(feature = "count")]
fn bytecode_never_roots_and_builtins_do() {
    use s3_gc::count::{self, Context};
    big_stack(|| {
        for w in workloads::all(Size::Test) {
            for &technique in Technique::ALL {
                let stack = RawStack::default();
                let loaded = Loaded::<RawStack>::new(w.program.clone()).expect("verified");
                let mut vm = Vm::new(&stack, Pacing::STRESS);
                let _ = count::take();
                let result = vm.run(&loaded, technique, &w.args).expect("runs");
                assert_eq!(vm.digest(result), w.expected);
                let roots = count::take();
                let stats = vm.take_stats();
                assert_eq!(
                    stats.instructions(),
                    s3_vm::reference::run(&w.program, &w.args).instructions,
                    "{}, {technique}: the same instructions as the reference",
                    w.name
                );
                assert_eq!(
                    roots.in_context(Context::Interpreter),
                    0,
                    "{}, {technique}: bytecode rooted something",
                    w.name
                );
                assert_eq!(roots.in_context(Context::Coercion), 0, "hooks root nothing");
                if w.name == "w3" {
                    assert!(roots.in_context(Context::Builtin) > 0);
                    assert!(stats.builtin_calls > 0 && stats.coercions > 0);
                }
            }
        }
    });
}
