//! Tests for the Chapter 20 challenges: `cargo test --test challenges_ch20`.
use carafe::{ClassFile, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn parse(class: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile("Reflect", class)).unwrap()).unwrap()
}

// @challenge 1
#[test]
fn predict_the_members() {
    let planet = parse("Reflect$Planet");
    let methods: Vec<&str> = planet
        .methods
        .iter()
        .filter(|m| !m.name.starts_with('<'))
        .map(|m| m.name.as_str())
        .collect();
    // values and valueOf, which javac writes for every enum, $values, which
    // fills the array, and inner, the one the source declares.
    assert_eq!(methods, ["values", "valueOf", "inner", "$values"]);
    let counter = parse("Reflect$Counter");
    let constructors = counter.methods.iter().filter(|m| m.name == "<init>").count();
    assert_eq!(constructors, 2);
}

// @challenge 2
use carafe::modifier_string;

#[test]
fn modifiers_as_java_prints_them() {
    assert_eq!(modifier_string(0x0009), "public static");
    assert_eq!(modifier_string(0x0001 | 0x0010 | 0x0400), "public abstract final");
    assert_eq!(
        modifier_string(0x0002 | 0x0020 | 0x0100),
        "private synchronized native"
    );
    assert_eq!(modifier_string(0x0601), "public abstract interface");
}

// @challenge 3
use carafe::signature_classes;

#[test]
fn what_reflection_loads() {
    let counter = parse("Reflect$Counter");
    let classes = signature_classes(&counter);
    assert_eq!(classes.iter().collect::<Vec<_>>(), ["java/lang/String"]);
    let app = compile("Reflect", "Reflect").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main("Reflect").unwrap();
    for planet_class in signature_classes(&parse("Reflect$Planet")) {
        assert!(vm.loaded.contains(&planet_class), "{planet_class}");
    }
}
