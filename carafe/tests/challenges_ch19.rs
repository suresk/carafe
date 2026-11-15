//! Tests for the Chapter 19 challenges: `cargo test --test challenges_ch19`.
use carafe::{ClassFile, Jdk, Ref, Value, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn kinds() -> Vm {
    let app = compile("Kinds", "Kinds").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main("Kinds").unwrap();
    vm
}

// @challenge 1
#[test]
fn predict_the_hash_codes() {
    let vm = kinds();
    let output = String::from_utf8(vm.stdout).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    // Circle(1.0): one component, so 0 * 31 + Double.hashCode(1.0).
    let bits = 1.0f64.to_bits();
    let circle = (bits ^ (bits >> 32)) as i32;
    // Square(7): 0 * 31 + Integer.hashCode(7).
    assert_eq!(lines[5], format!("{circle} 7"));
}

// @challenge 2
use carafe::record_components;

#[test]
fn record_components_from_the_class_file() {
    let file =
        ClassFile::parse(&std::fs::read(compile("Kinds", "Kinds$Group")).unwrap())
            .unwrap();
    let components = record_components(&file).unwrap();
    let expected = [("name", "Ljava/lang/String;"), ("members", "Ljava/util/List;")];
    let expected: Vec<(String, String)> =
        expected.iter().map(|(n, d)| (n.to_string(), d.to_string())).collect();
    assert_eq!(components, expected);
    let kinds =
        ClassFile::parse(&std::fs::read(compile("Kinds", "Kinds")).unwrap()).unwrap();
    assert!(record_components(&kinds).unwrap().is_empty());
}

// @challenge 3
use carafe::enum_switch;

#[test]
fn an_enum_switch() {
    let mut vm = kinds();
    let color = vm.load("Kinds$Color").unwrap();
    let slot = color.static_slot("BLUE", "LKinds$Color;").unwrap();
    let Value::Ref(blue) = color.statics.borrow()[slot] else { panic!() };
    assert_eq!(enum_switch(&mut vm, &["RED", "BLUE"], blue, 0).unwrap(), 1);
    assert_eq!(enum_switch(&mut vm, &["RED", "BLUE"], blue, 2).unwrap(), 2);
    assert_eq!(enum_switch(&mut vm, &["RED", "GREEN"], blue, 0).unwrap(), 2);
    assert_eq!(enum_switch(&mut vm, &["RED"], Ref::NULL, 0).unwrap(), -1);
}
