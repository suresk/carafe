//! Tests for the Chapter 8 challenges: `cargo test --test challenges_ch08`.
use carafe::{ClassFile, ClassPath, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn parse(source: &str, class: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile(source, class)).unwrap()).unwrap()
}

// @challenge 1
#[test]
fn predict_the_order() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch08-challenge");
    std::fs::create_dir_all(&dir).unwrap();
    for class in
        ["Loading", "Geometry", "Figure", "Labeled", "Rectangle", "Window", "Unused"]
    {
        std::fs::copy(compile("Loading", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir] };
    vm.load("Window").unwrap();
    assert_eq!(vm.loaded, ["Labeled", "Figure", "Rectangle", "Window"]);
    vm.load("Rectangle").unwrap();
    vm.load("Geometry").unwrap();
    assert_eq!(vm.loaded.len(), 5);
}

// @challenge 2
use carafe::references;

#[test]
fn the_classes_a_class_names() {
    let loading = references(&parse("Loading", "Loading")).unwrap();
    let names: Vec<&str> = loading.iter().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "Geometry",
            "Rectangle",
            "Unused",
            "Window",
            "java/lang/Object",
            "java/lang/String"
        ]
    );
    let shapes = references(&parse("Shapes", "Shapes")).unwrap();
    assert!(shapes.contains("java/io/PrintStream") && shapes.contains("Shape"));
}

// @challenge 3
use carafe::nestmates;

#[test]
fn nestmates_check_both_ways() {
    let lambdas = parse("Lambdas", "Lambdas");
    let point = parse("Lambdas", "Lambdas$Point");
    assert!(nestmates(&lambdas, &point).unwrap());
    assert!(!nestmates(&point, &lambdas).unwrap());
    let hello = parse("Hello", "Hello");
    assert!(!nestmates(&hello, &point).unwrap()); // Point names Lambdas, not Hello
}
