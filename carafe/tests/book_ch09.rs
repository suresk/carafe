//! The examples in Chapter 9: `cargo test --test book_ch09`.
use carafe::{ClassPath, Jdk, State, Value, Vm};
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
/// The classes `Init.java` declares.
const CLASSES: &str = "Init Trace Base Derived Constants9 Ping Pong WithDefault \
                       WithoutDefault Implementer Fragile";

/// A VM whose class path is a directory of its own, for the test `test`,
/// holding the classes of `Init.java`; and that directory.
fn vm(test: &str) -> (Vm, PathBuf) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch09").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    for class in CLASSES.split_whitespace() {
        std::fs::copy(compile("Init", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir.clone()] };
    (vm, dir)
}

/// Runs a static method of `Init` that takes no arguments.
fn scenario(
    vm: &mut Vm,
    name: &str,
    descriptor: &str,
) -> carafe::Result<Option<Value>> {
    let init = vm.load("Init")?;
    vm.invoke(&init, name, descriptor, &[])
}
// @end

#[test]
fn initialization_order() {
    // @listing order-test
    let (mut vm, _) = vm("order");
    let trace = |vm: &mut Vm, name| scenario(vm, name, "()J").unwrap();
    assert_eq!(trace(&mut vm, "superclassFirst"), Some(Value::Long(123)));
    assert_eq!(trace(&mut vm, "constantsAreNotAccesses"), Some(Value::Long(455)));
    assert_eq!(trace(&mut vm, "cycle"), Some(Value::Long(21)));
    assert_eq!(trace(&mut vm, "interfaces"), Some(Value::Long(68)));

    let derived = vm.load("Derived").unwrap();
    assert_eq!(*derived.statics.borrow(), [Value::Int(2), Value::Int(3)]);
    let without = vm.load("WithoutDefault").unwrap();
    assert_eq!(without.state.get(), State::Uninitialized);
    // @end
}

#[test]
fn a_failing_initializer() {
    // @listing failing-test
    let (mut vm, _) = vm("failing");
    let first = scenario(&mut vm, "failing", "()I").unwrap_err();
    assert_eq!(first.to_string(), "java.lang.ExceptionInInitializerError");
    let again = scenario(&mut vm, "failing", "()I").unwrap_err();
    assert_eq!(
        again.to_string(),
        "java.lang.NoClassDefFoundError: Could not initialize class Fragile"
    );
    assert_eq!(vm.load("Fragile").unwrap().state.get(), State::Erroneous);
    // @end
}

#[test]
fn the_same_as_hotspot() {
    // @listing hotspot-test
    let (mut vm, dir) = vm("hotspot");
    let printed = Jdk::find().unwrap().java(&dir, "Init").unwrap();
    let mut ours = Vec::new();
    let traces =
        ["superclassFirst", "constantsAreNotAccesses", "cycle", "interfaces"];
    for name in traces {
        let Some(Value::Long(trace)) = scenario(&mut vm, name, "()J").unwrap() else {
            panic!("{name} returned no long")
        };
        ours.push(trace.to_string());
    }
    for _ in 0..2 {
        ours.push(scenario(&mut vm, "failing", "()I").unwrap_err().to_string());
    }
    assert_eq!(ours, printed.lines().collect::<Vec<_>>());
    // @end
}
