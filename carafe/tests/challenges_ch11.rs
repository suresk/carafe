//! Tests for the Chapter 11 challenges: `cargo test --test challenges_ch11`.
use carafe::{Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn vm(test: &str) -> Vm {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch11-challenges").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    for class in
        "Dispatch Dispatch$Inner Animal Dog Puppy Shape2 Circle2".split_whitespace()
    {
        std::fs::copy(compile("Dispatch", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &dir).unwrap()
}

// @challenge 1
#[test]
fn predict_the_constructors() {
    let mut vm = vm("predict");
    vm.run_main("Dispatch").unwrap();
    let output = String::from_utf8(vm.stdout).unwrap();
    let first: Vec<&str> = output.lines().take(6).collect();
    assert_eq!(
        first,
        ["Animal()", "Animal()", "Dog()", "Animal()", "Dog()", "Puppy()"]
    );
}

// @challenge 2
use carafe::vtable;

#[test]
fn a_vtable() {
    let mut vm = vm("vtable");
    let puppy = vm.load("Puppy").unwrap();
    let table = vtable(&puppy);
    let find =
        |key: &str| table.iter().find(|(k, _)| k == key).map(|(_, c)| c.as_str());
    assert_eq!(find("speak()V"), Some("Puppy"));
    assert_eq!(find("origin()V"), Some("Puppy"));
    assert_eq!(find("describe()V"), Some("Animal"));
    assert_eq!(find("secret()V"), None); // private
    assert_eq!(find("toString()Ljava/lang/String;"), Some("java/lang/Object"));
    let object = vtable(&vm.load("java/lang/Object").unwrap()).len();
    let animal = vtable(&vm.load("Animal").unwrap()).len();
    assert_eq!(animal - object, 4); // speak, origin, secretly, describe
    assert_eq!(table.len(), animal); // Dog and Puppy only override
}
