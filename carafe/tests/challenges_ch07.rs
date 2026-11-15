//! Tests for the Chapter 7 challenges: `cargo test --test challenges_ch07`.
use carafe::{Class, ClassFile, Jdk, Layout, Ref, Value, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn parse(name: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile(name, name)).unwrap()).unwrap()
}

fn offsets(layout: &Layout) -> Vec<(&str, u32)> {
    layout.fields.iter().map(|f| (f.name.as_str(), f.offset)).collect()
}

// @challenge 1
#[test]
fn predict_the_layout() {
    let layouts = Class::new(parse("Layouts"));
    assert_eq!(
        offsets(&layouts.layout),
        [("y", 8), ("next", 16), ("w", 20), ("x", 24), ("tag", 26), ("z", 28)]
    );
    assert_eq!(layouts.layout.size, 29); // 32 with rounding
}

// @challenge 2
use carafe::packed_layout;

#[test]
fn packing_fills_the_gaps() {
    // A superclass whose objects end after one int field, at 12.
    let superclass = Layout { size: 12, fields: Vec::new() };
    let plain = Layout::new(&parse("Layouts"), &superclass);
    assert_eq!(plain.size, 37); // y at 16: 4 bytes wasted at 12
    let packed = packed_layout(&parse("Layouts"), &superclass);
    assert_eq!(
        offsets(&packed),
        [("next", 12), ("y", 16), ("w", 24), ("x", 28), ("tag", 30), ("z", 32)]
    );
    assert_eq!(packed.size, 33);
    let mixed = packed_layout(&parse("Mixed"), &Layout::OBJECT);
    assert_eq!(mixed.size, 38); // nothing to gain after an 8-byte header
}

// @challenge 3
use carafe::objects;

#[test]
fn walking_the_heap() {
    let node = Rc::new(Class::new(parse("Node")));
    let mixed = Rc::new(Class::new(parse("Mixed")));
    let mut vm = Vm::new();
    vm.invoke(&node, "range", "(I)LNode;", &[Value::Int(3)]).unwrap();
    vm.invoke(&mixed, "make", "()LMixed;", &[]).unwrap();
    vm.invoke(&node, "range", "(I)LNode;", &[Value::Int(1)]).unwrap();
    let found = objects(&vm.heap);
    assert_eq!(found, [Ref(8), Ref(24), Ref(40), Ref(56), Ref(96)]);
    assert!(Rc::ptr_eq(vm.heap.class_of(found[3]), &mixed));
}
