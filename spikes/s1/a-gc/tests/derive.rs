//! `#[derive(Trace)]` across type shapes: enums, tuple and unit structs, type
//! parameters, and `require_static` fields.
#![forbid(unsafe_code)]

use s1_a_gc::{Arena, Gc, Lock, Rootable, Trace};

#[derive(Trace)]
enum Shape<'gc> {
    Leaf,
    Pair(Gc<'gc, Shape<'gc>>, Gc<'gc, Shape<'gc>>),
    Named {
        label: String,
        child: Lock<Option<Gc<'gc, Shape<'gc>>>>,
    },
}

#[derive(Trace)]
struct Wrapper<T>(T);

#[derive(Trace)]
struct Unit;

#[derive(Trace)]
struct Statics {
    #[trace(require_static)]
    label: Box<str>,
    count: u32,
}

/// Checked at compile time: a wrong `NEEDS_TRACE` fails the build.
const _: () = {
    assert!(!<Unit as Trace>::NEEDS_TRACE);
    assert!(
        !<Statics as Trace>::NEEDS_TRACE,
        "require_static fields are excluded"
    );
    assert!(!<Wrapper<u32> as Trace>::NEEDS_TRACE);
    assert!(<Wrapper<Gc<'static, Unit>> as Trace>::NEEDS_TRACE);
    assert!(<Shape<'static> as Trace>::NEEDS_TRACE);
};

#[test]
fn every_variant_of_a_derived_enum_is_traced() {
    struct Root;
    impl Rootable for Root {
        type Root<'gc> = Option<Gc<'gc, Shape<'gc>>>;
    }
    let mut arena = Arena::<Root>::new(|_| None);
    arena.mutate_root(|mc, root| {
        let (l1, l2) = (Gc::new(mc, Shape::Leaf), Gc::new(mc, Shape::Leaf));
        let pair = Gc::new(mc, Shape::Pair(l1, l2));
        let named = Gc::new(
            mc,
            Shape::Named {
                label: "n".into(),
                child: Lock::new(Some(pair)),
            },
        );
        let l3 = Gc::new(mc, Shape::Leaf);
        Gc::new(mc, Shape::Pair(l3, l3)); // unreachable
        *root = Some(named);
    });
    arena.collect_all();
    assert_eq!(arena.metrics().objects, 4, "named, pair, and both leaves");

    arena.mutate(|_, root| {
        let Shape::Named { label, child } = &**root.as_ref().expect("rooted") else {
            panic!("root is Named");
        };
        assert_eq!(label, "n");
        assert!(matches!(*child.get().expect("child"), Shape::Pair(..)));
    });
}

#[test]
fn require_static_fields_are_stored_untraced() {
    struct Root;
    impl Rootable for Root {
        type Root<'gc> = Gc<'gc, Statics>;
    }
    let mut arena = Arena::<Root>::new(|mc| {
        Gc::new(
            mc,
            Statics {
                label: "kept".into(),
                count: 3,
            },
        )
    });
    arena.collect_all();
    arena.mutate(|_, statics| assert_eq!((&*statics.label, statics.count), ("kept", 3)));
}
