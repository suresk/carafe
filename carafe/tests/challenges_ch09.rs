//! Tests for the Chapter 9 challenges: `cargo test --test challenges_ch09`.
use carafe::{ClassFile, ClassPath, Jdk, Value, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

const CLASSES: &str = "Init Trace Base Derived Constants9 Ping Pong WithDefault \
                       WithoutDefault Implementer Fragile";

fn vm(test: &str) -> Vm {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch09-challenges").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    for class in CLASSES.split_whitespace() {
        std::fs::copy(compile("Init", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir] };
    vm
}

// @challenge 1
#[test]
fn predict_the_trace() {
    let mut vm = vm("predict");
    let init = vm.load("Init").unwrap();
    let trace = vm.invoke(&init, "inheritedField", "()J", &[]).unwrap();
    assert_eq!(trace, Some(Value::Long(11))); // Base only, then the value 1
    let derived = vm.load("Derived").unwrap();
    assert_eq!(derived.state.get(), carafe::State::Uninitialized);
}

// @challenge 2
use carafe::static_values;

#[test]
fn the_statics_of_a_class() {
    let mut vm = vm("statics");
    let init = vm.load("Init").unwrap();
    vm.invoke(&init, "cycle", "()J", &[]).unwrap();
    let ping = static_values(&vm.load("Ping").unwrap());
    assert_eq!(ping, [("p".to_string(), Value::Int(2))]);
    let constants = vm.load("Constants9").unwrap();
    let values = static_values(&constants);
    assert_eq!(values[0], ("K".to_string(), Value::Int(0))); // not initialized yet
}

// @challenge 3
use carafe::init_cycles;

#[test]
fn initialization_cycles() {
    let files: Vec<ClassFile> = CLASSES
        .split_whitespace()
        .map(|c| {
            ClassFile::parse(&std::fs::read(compile("Init", c)).unwrap()).unwrap()
        })
        .collect();
    let cycles = init_cycles(&files).unwrap();
    assert_eq!(
        cycles.into_iter().collect::<Vec<_>>(),
        [("Ping".into(), "Pong".into())]
    );
}
