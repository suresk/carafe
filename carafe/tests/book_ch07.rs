//! The examples in Chapter 7: `cargo test --test book_ch07`.
use carafe::{Class, ClassFile, FieldType, Jdk, Ref, Value, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Compiles `java/<source>.java` with the pinned JDK and returns the path of
/// the class file for `class`, one of the classes the source declares.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

// @listing helpers
/// Loads the class `name`, compiled from `java/<name>.java`, for running.
fn class(name: &str) -> Rc<Class> {
    let bytes = std::fs::read(compile(name, name)).unwrap();
    Rc::new(Class::new(ClassFile::parse(&bytes).unwrap()))
}

/// The reference a method returned.
fn reference(result: Option<Value>) -> Ref {
    match result {
        Some(Value::Ref(r)) => r,
        other => panic!("not a reference: {other:?}"),
    }
}
// @end

#[test]
fn layouts() {
    // @listing layout-test
    let node = class("Node");
    assert_eq!(node.layout.size, 16); // header 8, int 4, reference 4

    let mixed = class("Mixed");
    let offsets: Vec<(&str, u32)> =
        mixed.layout.fields.iter().map(|f| (f.name.as_str(), f.offset)).collect();
    assert_eq!(
        offsets,
        [
            ("l", 8),
            ("d", 16),
            ("o", 24),
            ("i", 28),
            ("s", 32),
            ("c", 34),
            ("b", 36),
            ("z", 37)
        ]
    );
    assert_eq!(mixed.layout.size, 38); // allocated as 40
    // @end
}

#[test]
fn a_linked_list() {
    // @listing list-test
    let node = class("Node");
    let mut vm = Vm::new();
    let list =
        reference(vm.invoke(&node, "range", "(I)LNode;", &[Value::Int(5)]).unwrap());
    assert_eq!(list, Ref(72)); // the last of five 16-byte nodes after offset 8
    assert_eq!(vm.heap.bytes().len(), 8 + 5 * 16);

    let sum = vm.invoke(&node, "sum", "(LNode;)I", &[Value::Ref(list)]).unwrap();
    assert_eq!(sum, Some(Value::Int(15)));

    let reversed = vm.invoke(&node, "reverse", "(LNode;)LNode;", &[Value::Ref(list)]);
    let reversed = reference(reversed.unwrap());
    assert_eq!(reversed, Ref(8)); // node 5, the first one allocated
    assert_eq!(vm.heap.read(reversed.0 + 8, &FieldType::Int), Value::Int(5));
    let sum = vm.invoke(&node, "sum", "(LNode;)I", &[Value::Ref(reversed)]).unwrap();
    assert_eq!(sum, Some(Value::Int(15)));
    assert_eq!(vm.heap.bytes().len(), 88); // reversing allocates nothing
    // @end
}

#[test]
fn the_heap_byte_by_byte() {
    // @listing bytes-test
    let node = class("Node");
    let mut vm = Vm::new();
    let list =
        reference(vm.invoke(&node, "range", "(I)LNode;", &[Value::Int(2)]).unwrap());
    assert_eq!(list, Ref(24));
    #[rustfmt::skip]
    assert_eq!(
        vm.heap.bytes()[8..40],
        [
            0, 0, 0, 0,  8, 0, 0, 0,   2, 0, 0, 0,  0, 0, 0, 0, // at 8: Node(2, null)
            0, 0, 0, 0, 24, 0, 0, 0,   1, 0, 0, 0,  8, 0, 0, 0, // at 24: Node(1, @8)
        ]
    );
    assert_eq!(vm.heap.hash(list), 24);
    assert!(Rc::ptr_eq(vm.heap.class_of(list), &node));
    // @end
}

#[test]
fn every_kind_of_field() {
    // @listing mixed-test
    let mixed = class("Mixed");
    let mut vm = Vm::new();
    let m = reference(vm.invoke(&mixed, "make", "()LMixed;", &[]).unwrap());
    assert_eq!(m, Ref(8));
    #[rustfmt::skip]
    assert_eq!(
        vm.heap.bytes()[8..48],
        [
            0, 0, 0, 0,  8, 0, 0, 0,                  // header: class 0, hash 8
            8, 7, 6, 5, 4, 3, 2, 1,                   // l at 8, little-endian
            0, 0, 0, 0, 0, 0, 0xf0, 0x3f,             // d at 16: 1.0
            8, 0, 0, 0,  7, 0, 0, 0,                  // o at 24 (itself), i at 28
            0x2c, 1,  0x41, 0,  0xff, 1,  0, 0,       // s, c, b, z; 2 bytes of padding
        ]
    );
    // @end
}

#[test]
fn narrow_stores() {
    // @listing narrow-test
    let mixed = class("Mixed");
    let mut vm = Vm::new();
    let m = reference(vm.invoke(&mixed, "make", "()LMixed;", &[]).unwrap());
    vm.heap.write(m.0 + 36, &FieldType::Byte, Value::Int(300)).unwrap();
    assert_eq!(vm.heap.read(m.0 + 36, &FieldType::Byte), Value::Int(44));
    vm.heap.write(m.0 + 37, &FieldType::Boolean, Value::Int(2)).unwrap();
    assert_eq!(vm.heap.read(m.0 + 37, &FieldType::Boolean), Value::Int(0));
    let wrong = vm.heap.write(m.0 + 28, &FieldType::Int, Value::Long(1));
    assert_eq!(wrong.unwrap_err().to_string(), "VerifyError: storing Long(1) as int");
    // @end
}

#[test]
fn null() {
    // @listing null-test
    let node = class("Node");
    let mut vm = Vm::new();
    let null = Value::Ref(Ref::NULL);
    assert_eq!(
        vm.invoke(&node, "sum", "(LNode;)I", &[null]).unwrap(),
        Some(Value::Int(0))
    );
    let error = vm.invoke(&node, "first", "(LNode;)I", &[null]).unwrap_err();
    assert_eq!(error.to_string(), "java.lang.NullPointerException");

    let one =
        reference(vm.invoke(&node, "range", "(I)LNode;", &[Value::Int(1)]).unwrap());
    let same =
        |vm: &mut Vm, a, b| vm.invoke(&node, "same", "(LNode;LNode;)Z", &[a, b]);
    let (a, b) = (Value::Ref(one), Value::Ref(Ref::NULL));
    assert_eq!(same(&mut vm, a, a).unwrap(), Some(Value::Int(1)));
    assert_eq!(same(&mut vm, a, b).unwrap(), Some(Value::Int(0)));
    // @end
}
