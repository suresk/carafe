//! Tests for the Chapter 12 challenges: `cargo test --test challenges_ch12`.
use carafe::{FieldType, Jdk, Ref, Value, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn sorting() -> Vm {
    let app = compile("Sorting", "Sorting").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap()
}

// @challenge 1
#[test]
fn predict_the_sizes() {
    let mut vm = sorting();
    let mut grow = |ty: FieldType, length: u32| {
        let before = vm.heap.bytes().len();
        vm.heap.new_array(&ty, length).unwrap();
        vm.heap.bytes().len() - before
    };
    assert_eq!(grow(FieldType::Long, 3), 40);
    assert_eq!(grow(FieldType::Boolean, 5), 24);
    assert_eq!(grow(FieldType::Int, 0), 16);
    let sorting = vm.load("Sorting").unwrap();
    let before = vm.heap.bytes().len();
    vm.invoke(&sorting, "grid", "(II)[[J", &[Value::Int(2), Value::Int(3)]).unwrap();
    assert_eq!(vm.heap.bytes().len() - before, 24 + 2 * 40);
}

// @challenge 2
use carafe::objects_and_arrays;

#[test]
fn walking_arrays_too() {
    let mut vm = sorting();
    let sorting = vm.load("Sorting").unwrap();
    vm.invoke(&sorting, "grid", "(II)[[J", &[Value::Int(2), Value::Int(3)]).unwrap();
    let units: Vec<u16> = "hi".encode_utf16().collect();
    vm.new_string(&units).unwrap();
    let found = objects_and_arrays(&vm.heap);
    assert_eq!(found, [Ref(8), Ref(32), Ref(72), Ref(112), Ref(136)]);
    assert!(vm.heap.is_array(found[1]) && !vm.heap.is_array(found[3]));
}
