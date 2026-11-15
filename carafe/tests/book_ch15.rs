//! The examples in Chapter 15: `cargo test --test book_ch15`.
use carafe::{Jdk, Value, Vm};
use std::path::{Path, PathBuf};

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
/// Runs `program` on carafe and on HotSpot, and returns carafe's VM, what it
/// printed, and what HotSpot printed.
fn both(program: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java(&app, program).unwrap();
    (vm, ours, theirs)
}
// @end

#[test]
fn the_collections() {
    // @listing collections-test
    let (vm, ours, theirs) = both("Collections15");
    assert_eq!(ours, theirs);
    let lines: Vec<&str> = ours.lines().collect();
    assert_eq!(lines[0], "{the=3, mat=1, cat=1, sat=1, end=1, on=1}"); // HashMap order
    assert_eq!(lines[1], "{cat=1, end=1, mat=1, on=1, sat=1, the=3}"); // TreeMap order
    let loaded = |name: &str| vm.loaded.iter().any(|c| c == name);
    assert!(loaded("java/util/HashMap$Node") && loaded("java/util/TreeMap$Entry"));
    // @end
}

#[test]
fn class_objects() {
    // @listing classes-test
    let (mut vm, ours, theirs) = both("Classes");
    assert_eq!(ours, theirs);
    let read = |vm: &Vm, mirror: carafe::Ref, name: &str, ty: &str| {
        let class = vm.heap.class_of(mirror).clone();
        let slot = class.field(name, ty).unwrap();
        vm.heap.read(mirror.0 + slot.offset, &slot.ty)
    };
    let grid = vm.load("[[Ljava/lang/String;").unwrap();
    let mirror = vm.mirror(&grid).unwrap();
    let Value::Ref(component) =
        read(&vm, mirror, "componentType", "Ljava/lang/Class;")
    else {
        panic!()
    };
    assert_eq!(
        vm.class_of_mirror(component).unwrap().file.this_class,
        "[Ljava/lang/String;"
    );
    let modifiers = read(&vm, mirror, "modifiers", "C");
    assert_eq!(modifiers, Value::Int(0x0411)); // public final abstract
    // @end
}

#[test]
fn deterministic_hash_codes() {
    // @listing hash-test
    let (first, _, _) = both("Collections15");
    let (second, _, _) = both("Collections15");
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.heap.bytes(), second.heap.bytes()); // the same heap, byte for byte
    // @end
}
