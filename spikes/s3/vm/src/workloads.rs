//! The four workloads of spike S3 (tracker §10.3), in bytecode, each with a
//! native Rust model of its expected result.
//!
//! | Workload | Exercises |
//! |---|---|
//! | W1, arithmetic loop | dispatch only: Collatz step counts, no allocation |
//! | W2, object churn | allocation, slot access, frequent collection: linked lists built and walked |
//! | W3, callback-heavy builtins | `map`, `sortBy` and `forEach` with closures over an environment, and objects converted through their hook |
//! | W4, tree traversal and mutation | a random tree of objects, recursive sums, subtree moves with ancestor checks |
//!
//! Register discipline: every function allocates its variables first and its
//! call registers (callee and arguments) last, because a call clobbers every
//! register above its callee.
#![expect(
    clippy::many_single_char_names,
    reason = "assembly style: short register names, as in the bytecode docs"
)]

use s3_gc::Value;

use crate::builder::{FnBuilder, FuncId, ProgramBuilder, Reg};
use crate::builtins::Builtin;
use crate::bytecode::Program;
use crate::digest::Digest;

/// How big a run is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// Small enough for every configuration, stress pacing and Miri.
    Test,
    /// What the benchmarks time: tens of milliseconds per run.
    Bench,
}

/// A program, its arguments, and the digest its result must have.
#[derive(Clone, Debug)]
pub struct Workload {
    pub name: &'static str,
    pub program: Program,
    pub args: Vec<Value>,
    pub expected: u64,
}

/// W1–W4 at `size`.
#[must_use]
pub fn all(size: Size) -> Vec<Workload> {
    let test = size == Size::Test;
    vec![
        w1_arith(if test { 300 } else { 10_000 }),
        w2_churn(if test { 4 } else { 300 }, if test { 50 } else { 1000 }),
        w3_callbacks(if test { 60 } else { 5000 }),
        w4_tree(if test { 60 } else { 5000 }, if test { 60 } else { 5000 }),
    ]
}

/// One workload by name (`w1` … `w4`).
#[must_use]
pub fn named(name: &str, size: Size) -> Option<Workload> {
    all(size).into_iter().find(|w| w.name == name)
}

fn int(n: u32) -> Value {
    Value::int(i32::try_from(n).expect("workload sizes fit i32"))
}

fn build(b: ProgramBuilder) -> Program {
    b.build().expect("workloads are valid programs")
}

/// `dst = src % k` for a constant `k`, through a scratch register.
fn rem_const(f: &mut FnBuilder, dst: Reg, src: Reg, k: i32, scratch: Reg) {
    f.int(scratch, k);
    f.rem(dst, src, scratch);
}

// ----- W1: arithmetic -----

/// The total number of Collatz steps for `1..=n`.
#[must_use]
pub fn w1_arith(n: u32) -> Workload {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let mut f = b.body(main, 1);
    let n_reg = f.param(0);
    let [total, i, x, steps, t, r, zero, one, two, three] = f.regs();
    f.int(total, 0);
    f.int(i, 1);
    f.int(zero, 0);
    f.int(one, 1);
    f.int(two, 2);
    f.int(three, 3);
    let (outer, done, inner, inner_end, odd, next) = (
        f.label(),
        f.label(),
        f.label(),
        f.label(),
        f.label(),
        f.label(),
    );
    f.bind(outer);
    f.le(t, i, n_reg);
    f.jump_if_not(t, done);
    f.mov(x, i);
    f.int(steps, 0);
    f.bind(inner);
    f.eq(t, x, one);
    f.jump_if(t, inner_end);
    f.rem(r, x, two);
    f.eq(t, r, zero);
    f.jump_if_not(t, odd);
    f.div(x, x, two);
    f.jump(next);
    f.bind(odd);
    f.mul(x, x, three);
    f.add_imm(x, x, 1);
    f.bind(next);
    f.add_imm(steps, steps, 1);
    f.jump(inner);
    f.bind(inner_end);
    f.add(total, total, steps);
    f.add_imm(i, i, 1);
    f.jump(outer);
    f.bind(done);
    f.ret(total);
    b.define(f);

    let mut expected = 0_u64;
    for start in 1..=u64::from(n) {
        let mut x = start;
        while x != 1 {
            x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
            expected += 1;
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "small totals")]
    let expected = Digest::of_number(expected as f64);
    Workload {
        name: "w1",
        program: build(b),
        args: vec![int(n)],
        expected,
    }
}

// ----- W2: object churn -----

const W2_MODULUS: i32 = 1_000_000_007;

/// `rounds` times: build a linked list of `k` objects, then walk it, summing
/// two slots of each.
#[must_use]
pub fn w2_churn(rounds: u32, k: u32) -> Workload {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let mut f = b.body(main, 2);
    let (rounds_reg, k_reg) = (f.param(0), f.param(1));
    let [total, round, head, i, o, s, p, v, t, undef, modulus] = f.regs();
    f.int(total, 0);
    f.int(round, 0);
    f.undefined(undef);
    f.int(modulus, W2_MODULUS);
    let (outer, done, build_loop, build_end, walk, walk_end) = (
        f.label(),
        f.label(),
        f.label(),
        f.label(),
        f.label(),
        f.label(),
    );
    f.bind(outer);
    f.lt(t, round, rounds_reg);
    f.jump_if_not(t, done);
    f.undefined(head);
    f.int(i, 0);
    f.bind(build_loop);
    f.lt(t, i, k_reg);
    f.jump_if_not(t, build_end);
    f.new_object(o);
    f.set_slot(o, 0, i);
    f.set_slot(o, 1, head);
    f.set_slot(o, 2, round);
    f.mov(head, o);
    f.add_imm(i, i, 1);
    f.jump(build_loop);
    f.bind(build_end);
    f.int(s, 0);
    f.mov(p, head);
    f.bind(walk);
    f.eq(t, p, undef);
    f.jump_if(t, walk_end);
    f.get_slot(v, p, 0);
    f.add(s, s, v);
    f.get_slot(v, p, 2);
    f.add(s, s, v);
    f.get_slot(p, p, 1);
    f.jump(walk);
    f.bind(walk_end);
    f.add(total, total, s);
    f.rem(total, total, modulus);
    f.add_imm(round, round, 1);
    f.jump(outer);
    f.bind(done);
    f.ret(total);
    b.define(f);

    let mut expected = 0_u64;
    for round in 0..u64::from(rounds) {
        let k = u64::from(k);
        let s = k * (k - 1) / 2 + k * round;
        expected = (expected + s) % u64::from(W2_MODULUS.cast_unsigned());
    }
    #[expect(clippy::cast_precision_loss, reason = "below the modulus")]
    let expected = Digest::of_number(expected as f64);
    Workload {
        name: "w2",
        program: build(b),
        args: vec![int(rounds), int(k)],
        expected,
    }
}

// ----- W3: callback-heavy builtins -----

const W3_OFFSET: i32 = 7;
const W3_HASH: i32 = 1_000_003;

/// Arrays mapped, sorted and visited by builtins with closures, which read
/// and write their environment; then objects summed and sorted through their
/// coercion hook. Returns `[sum, boxed sum, checksum of the sorted numbers,
/// checksum of the sorted objects]`.
#[must_use]
#[expect(clippy::too_many_lines, reason = "an assembly listing")]
pub fn w3_callbacks(n: u32) -> Workload {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let double = b.declare("double");
    let compare = b.declare("compare");
    let sum = b.declare("sum");
    let make_box = b.declare("box");
    let value_of = b.declare("valueOf");
    let add_box = b.declare("addBox");

    // double(x, i) = x * 2 + env.0
    let mut f = b.body(double, 2);
    let (x, two, k) = (f.param(0), f.reg(), f.reg());
    f.int(two, 2);
    f.mul(x, x, two);
    f.get_env(k, 0);
    f.add(x, x, k);
    f.ret(x);
    b.define(f);

    // compare(a, b) = a - b, converting objects through their hooks
    let mut f = b.body(compare, 2);
    let (a, c) = (f.param(0), f.param(1));
    f.sub(a, a, c);
    f.ret(a);
    b.define(f);

    // sum(x, i): env.1 = env.1 + x
    let mut f = b.body(sum, 2);
    let (x, acc) = (f.param(0), f.reg());
    f.get_env(acc, 1);
    f.add(acc, acc, x);
    f.set_env(1, acc);
    f.ret(acc);
    b.define(f);

    // box(x, i): an object holding x, converted through valueOf (env.2)
    let mut f = b.body(make_box, 2);
    let (x, o, hook) = (f.param(0), f.reg(), f.reg());
    f.new_object(o);
    f.set_slot(o, 0, x);
    f.get_env(hook, 2);
    f.set_slot(o, 3, hook);
    f.ret(o);
    b.define(f);

    // valueOf(self) = self.0
    let mut f = b.body(value_of, 1);
    let (this, v) = (f.param(0), f.reg());
    f.get_slot(v, this, 0);
    f.ret(v);
    b.define(f);

    // addBox(b, i): env.3 = env.3 + b, which converts b through its hook
    let mut f = b.body(add_box, 2);
    let (bx, acc) = (f.param(0), f.reg());
    f.get_env(acc, 3);
    f.add(acc, acc, bx);
    f.set_env(3, acc);
    f.ret(acc);
    b.define(f);

    let mut f = b.body(main, 1);
    let n_reg = f.param(0);
    let [
        env,
        arr,
        doubled,
        boxes,
        i,
        v,
        t,
        scratch,
        chk,
        result,
        hook,
    ] = f.regs();
    let [callee, arg0, arg1] = f.regs();
    f.new_object(env);
    f.int(v, W3_OFFSET);
    f.set_slot(env, 0, v);
    f.int(v, 0);
    f.set_slot(env, 1, v);
    f.set_slot(env, 3, v);
    f.undefined(v);
    f.closure(hook, value_of, v);
    f.set_slot(env, 2, hook);
    // arr[i] = (i * 7919) % 1009
    f.new_array(arr, n_reg);
    counted_loop(&mut f, i, n_reg, t, |f| {
        f.int(scratch, 7919);
        f.mul(v, i, scratch);
        rem_const(f, v, v, 1009, scratch);
        f.set_index(arr, i, v);
    });
    let call2 = |f: &mut FnBuilder, builtin: Builtin, array: Reg, function: FuncId| {
        f.builtin(callee, builtin);
        f.mov(arg0, array);
        f.closure(arg1, function, env);
        f.call(callee, &[arg0, arg1]);
    };
    call2(&mut f, Builtin::Map, arr, double);
    f.mov(doubled, callee);
    call2(&mut f, Builtin::SortBy, doubled, compare);
    call2(&mut f, Builtin::ForEach, doubled, sum);
    call2(&mut f, Builtin::Map, arr, make_box);
    f.mov(boxes, callee);
    call2(&mut f, Builtin::ForEach, boxes, add_box);
    call2(&mut f, Builtin::SortBy, boxes, compare);
    // Positional checksums of the two sorted arrays.
    f.int(v, 4);
    f.new_array(result, v);
    f.get_slot(v, env, 1);
    f.int(t, 0);
    f.set_index(result, t, v);
    f.get_slot(v, env, 3);
    f.int(t, 1);
    f.set_index(result, t, v);
    for (slot, array, boxed) in [(2, doubled, false), (3, boxes, true)] {
        f.int(chk, 0);
        counted_loop(&mut f, i, n_reg, t, |f| {
            f.get_index(v, array, i);
            if boxed {
                f.get_slot(v, v, 0);
            }
            f.int(scratch, 31);
            f.mul(chk, chk, scratch);
            f.add(chk, chk, v);
            rem_const(f, chk, chk, W3_HASH, scratch);
        });
        f.int(t, slot);
        f.set_index(result, t, chk);
    }
    f.ret(result);
    b.define(f);

    let arr: Vec<i64> = (0..i64::from(n)).map(|i| i * 7919 % 1009).collect();
    let mut doubled: Vec<i64> = arr.iter().map(|x| x * 2 + i64::from(W3_OFFSET)).collect();
    doubled.sort_unstable(); // equal numbers are indistinguishable
    let mut sorted = arr.clone();
    sorted.sort_unstable();
    let checksum = |values: &[i64]| {
        values
            .iter()
            .fold(0, |chk, v| (chk * 31 + v) % i64::from(W3_HASH))
    };
    #[expect(clippy::cast_precision_loss, reason = "small sums")]
    let expected = Digest::of_numbers(&[
        doubled.iter().sum::<i64>() as f64,
        arr.iter().sum::<i64>() as f64,
        checksum(&doubled) as f64,
        checksum(&sorted) as f64,
    ]);
    Workload {
        name: "w3",
        program: build(b),
        args: vec![int(n)],
        expected,
    }
}

/// `for (i = 0; i < n; i++) body`, with `t` as scratch.
fn counted_loop(f: &mut FnBuilder, i: Reg, n: Reg, t: Reg, body: impl FnOnce(&mut FnBuilder)) {
    let (top, end) = (f.label(), f.label());
    f.int(i, 0);
    f.bind(top);
    f.lt(t, i, n);
    f.jump_if_not(t, end);
    body(f);
    f.add_imm(i, i, 1);
    f.jump(top);
    f.bind(end);
}

// ----- W4: tree traversal and mutation -----

/// Slots of a tree node.
const FIRST_CHILD: u8 = 0;
const NEXT_SIBLING: u8 = 1;
const PARENT: u8 = 2;
const VALUE: u8 = 3;
/// The ZX81's generator: products stay small integers.
const LCG_MUL: i32 = 75;
const LCG_ADD: i32 = 74;
const LCG_MOD: i32 = 65_537;

/// A random tree of `n` objects, built by attaching each node to a random
/// earlier one; then `moves` random subtree moves, each checked so that no
/// node moves under its own subtree. Returns `[depth-weighted sum before,
/// after, moves made]`.
#[must_use]
#[expect(clippy::too_many_lines, reason = "an assembly listing")]
pub fn w4_tree(n: u32, moves: u32) -> Workload {
    let mut b = ProgramBuilder::new();
    let main = b.declare("main");
    let random = b.declare("random");
    let append = b.declare("append");
    let detach = b.declare("detach");
    let contains = b.declare("contains");
    let weighted = b.declare("sum");

    // random(): env.0 = (env.0 * 75 + 74) % 65537
    let mut f = b.body(random, 0);
    let [x, k] = f.regs();
    f.get_env(x, 0);
    f.int(k, LCG_MUL);
    f.mul(x, x, k);
    f.int(k, LCG_ADD);
    f.add(x, x, k);
    rem_const(&mut f, x, x, LCG_MOD, k);
    f.set_env(0, x);
    f.ret(x);
    b.define(f);

    // append(parent, child): prepend child to parent's children
    let mut f = b.body(append, 2);
    let (parent, child, first) = (f.param(0), f.param(1), f.reg());
    f.set_slot(child, PARENT, parent);
    f.get_slot(first, parent, FIRST_CHILD);
    f.set_slot(child, NEXT_SIBLING, first);
    f.set_slot(parent, FIRST_CHILD, child);
    f.ret(child);
    b.define(f);

    // detach(node): unlink node from its parent's children
    let mut f = b.body(detach, 1);
    let node = f.param(0);
    let [p, c, next, t, undef] = f.regs();
    let (scan, unlink, clear) = (f.label(), f.label(), f.label());
    f.undefined(undef);
    f.get_slot(p, node, PARENT);
    f.get_slot(next, node, NEXT_SIBLING);
    f.get_slot(c, p, FIRST_CHILD);
    f.eq(t, c, node);
    f.jump_if_not(t, scan);
    f.set_slot(p, FIRST_CHILD, next);
    f.jump(clear);
    f.bind(scan); // c is an earlier sibling: find the one before node
    f.get_slot(t, c, NEXT_SIBLING);
    f.eq(t, t, node);
    f.jump_if(t, unlink);
    f.get_slot(c, c, NEXT_SIBLING);
    f.jump(scan);
    f.bind(unlink);
    f.set_slot(c, NEXT_SIBLING, next);
    f.bind(clear);
    f.set_slot(node, NEXT_SIBLING, undef);
    f.set_slot(node, PARENT, undef);
    f.ret(node);
    b.define(f);

    // contains(ancestor, node): whether node is ancestor or below it
    let mut f = b.body(contains, 2);
    let (ancestor, node) = (f.param(0), f.param(1));
    let [t, undef] = f.regs();
    let (up, yes, no) = (f.label(), f.label(), f.label());
    f.undefined(undef);
    f.bind(up);
    f.eq(t, node, ancestor);
    f.jump_if(t, yes);
    f.get_slot(node, node, PARENT);
    f.eq(t, node, undef);
    f.jump_if(t, no);
    f.jump(up);
    f.bind(yes);
    f.boolean(t, true);
    f.ret(t);
    f.bind(no);
    f.boolean(t, false);
    f.ret(t);
    b.define(f);

    // sum(node, depth) = node.value * depth + Σ sum(child, depth + 1)
    let mut f = b.body(weighted, 2);
    let (node, depth) = (f.param(0), f.param(1));
    let [s, c, t, undef] = f.regs();
    let [callee, arg0, arg1] = f.regs();
    let (children, done) = (f.label(), f.label());
    f.undefined(undef);
    f.get_slot(s, node, VALUE);
    f.mul(s, s, depth);
    f.get_slot(c, node, FIRST_CHILD);
    f.bind(children);
    f.eq(t, c, undef);
    f.jump_if(t, done);
    f.closure(callee, weighted, undef);
    f.mov(arg0, c);
    f.add_imm(arg1, depth, 1);
    f.call(callee, &[arg0, arg1]);
    f.add(s, s, callee);
    f.get_slot(c, c, NEXT_SIBLING);
    f.jump(children);
    f.bind(done);
    f.ret(s);
    b.define(f);

    let mut f = b.body(main, 2);
    let (n_reg, moves_reg) = (f.param(0), f.param(1));
    let [
        env,
        rng,
        nodes,
        root,
        i,
        node,
        t,
        before,
        made,
        m,
        a,
        target,
        scratch,
        result,
    ] = f.regs();
    let [callee, arg0, arg1] = f.regs();
    f.new_object(env);
    f.int(t, 1);
    f.set_slot(env, 0, t);
    f.closure(rng, random, env);
    f.new_array(nodes, n_reg);
    f.new_object(root);
    f.int(t, 0);
    f.set_slot(root, VALUE, t);
    f.set_index(nodes, t, root);
    // Each node attaches to a random earlier node.
    let (grow, grown) = (f.label(), f.label());
    f.int(i, 1);
    f.bind(grow);
    f.lt(t, i, n_reg);
    f.jump_if_not(t, grown);
    f.new_object(node);
    f.set_slot(node, VALUE, i);
    f.mov(callee, rng);
    f.call(callee, &[]);
    f.rem(t, callee, i);
    f.get_index(target, nodes, t);
    f.closure(callee, append, env);
    f.mov(arg0, target);
    f.mov(arg1, node);
    f.call(callee, &[arg0, arg1]);
    f.set_index(nodes, i, node);
    f.add_imm(i, i, 1);
    f.jump(grow);
    f.bind(grown);
    let sum_tree = |f: &mut FnBuilder, dst: Reg| {
        f.closure(callee, weighted, env);
        f.mov(arg0, root);
        f.int(arg1, 1);
        f.call(callee, &[arg0, arg1]);
        f.mov(dst, callee);
    };
    sum_tree(&mut f, before);
    // Random subtree moves: a (never the root) under target, unless target is in a's subtree.
    let (step, skip, finished) = (f.label(), f.label(), f.label());
    f.int(made, 0);
    f.int(m, 0);
    f.bind(step);
    f.lt(t, m, moves_reg);
    f.jump_if_not(t, finished);
    f.mov(callee, rng);
    f.call(callee, &[]);
    f.add_imm(scratch, n_reg, -1);
    f.rem(t, callee, scratch);
    f.add_imm(t, t, 1);
    f.get_index(a, nodes, t);
    f.mov(callee, rng);
    f.call(callee, &[]);
    f.rem(t, callee, n_reg);
    f.get_index(target, nodes, t);
    f.closure(callee, contains, env);
    f.mov(arg0, a);
    f.mov(arg1, target);
    f.call(callee, &[arg0, arg1]);
    f.jump_if(callee, skip);
    f.closure(callee, detach, env);
    f.mov(arg0, a);
    f.call(callee, &[arg0]);
    f.closure(callee, append, env);
    f.mov(arg0, target);
    f.mov(arg1, a);
    f.call(callee, &[arg0, arg1]);
    f.add_imm(made, made, 1);
    f.bind(skip);
    f.add_imm(m, m, 1);
    f.jump(step);
    f.bind(finished);
    f.int(t, 3);
    f.new_array(result, t);
    f.int(t, 0);
    f.set_index(result, t, before);
    sum_tree(&mut f, scratch);
    f.int(t, 1);
    f.set_index(result, t, scratch);
    f.int(t, 2);
    f.set_index(result, t, made);
    f.ret(result);
    b.define(f);

    Workload {
        name: "w4",
        program: build(b),
        args: vec![int(n), int(moves)],
        expected: w4_model(n, moves),
    }
}

/// W4 in plain Rust, with the same generator and the same choices.
fn w4_model(n: u32, moves: u32) -> u64 {
    #[derive(Clone, Copy)]
    struct Node {
        first_child: Option<usize>,
        next_sibling: Option<usize>,
        parent: Option<usize>,
    }
    let n = n as usize;
    let mut state: i64 = 1;
    let mut random = || {
        state = (state * i64::from(LCG_MUL) + i64::from(LCG_ADD)) % i64::from(LCG_MOD);
        usize::try_from(state).expect("non-negative")
    };
    let mut tree = vec![
        Node {
            first_child: None,
            next_sibling: None,
            parent: None,
        };
        n
    ];
    let append = |tree: &mut [Node], parent: usize, child: usize| {
        tree[child].parent = Some(parent);
        tree[child].next_sibling = tree[parent].first_child;
        tree[parent].first_child = Some(child);
    };
    for i in 1..n {
        let parent = random() % i;
        append(&mut tree, parent, i);
    }
    let sum = |tree: &[Node]| {
        let mut total = 0_i64;
        let mut stack = vec![(0_usize, 1_i64)];
        while let Some((node, depth)) = stack.pop() {
            total += i64::try_from(node).expect("small") * depth;
            let mut child = tree[node].first_child;
            while let Some(c) = child {
                stack.push((c, depth + 1));
                child = tree[c].next_sibling;
            }
        }
        total
    };
    let before = sum(&tree);
    let mut made = 0_u32;
    for _ in 0..moves {
        let a = 1 + random() % (n - 1);
        let target = random() % n;
        let mut cursor = Some(target);
        let mut inside = false;
        while let Some(c) = cursor {
            if c == a {
                inside = true;
                break;
            }
            cursor = tree[c].parent;
        }
        if inside {
            continue;
        }
        let parent = tree[a].parent.expect("only the root has no parent");
        if tree[parent].first_child == Some(a) {
            tree[parent].first_child = tree[a].next_sibling;
        } else {
            let mut c = tree[parent].first_child.expect("a has siblings before it");
            while tree[c].next_sibling != Some(a) {
                c = tree[c].next_sibling.expect("a is among the children");
            }
            tree[c].next_sibling = tree[a].next_sibling;
        }
        tree[a].next_sibling = None;
        tree[a].parent = None;
        append(&mut tree, target, a);
        made += 1;
    }
    let after = sum(&tree);
    #[expect(clippy::cast_precision_loss, reason = "small sums")]
    Digest::of_numbers(&[before as f64, after as f64, f64::from(made)])
}
