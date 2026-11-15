//! Tests for the Chapter 24 challenges: `cargo test --test challenges_ch24`.
use carafe::{Jdk, Vm};
use std::path::Path;

/// Compiles and runs the corpus program `program` on carafe.
fn run(program: &str) -> Vm {
    let jdk = Jdk::find().unwrap();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("corpus").join(program);
    jdk.javac(&corpus.join(format!("{program}.java")), &out).unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &out).unwrap();
    vm.stdin = b"4\n5\n33\n".to_vec();
    vm.run_main(program).unwrap();
    vm
}

// @challenge 1
#[test]
fn predict_the_biggest() {
    let programs = ["Hello", "Modest", "Strings", "Streams", "Enums"];
    let classes: Vec<usize> = programs.iter().map(|p| run(p).loaded.len()).collect();
    let most =
        programs[classes.iter().enumerate().max_by_key(|&(_, n)| n).unwrap().0];
    assert_eq!(most, "Streams");
}

// @challenge 2
use carafe::startup_classes;

#[test]
fn what_startup_costs() {
    let vm = run("Hello");
    let startup = startup_classes(&vm, "Hello");
    assert_eq!(startup.len(), 30); // every class Hello loads but Hello itself
    assert!(startup.iter().any(|c| c == "java/lang/LangAccess"));
    assert!(startup.iter().any(|c| c == "java/lang/System$In"));
}

// @challenge 3
use carafe::natives_by_class;

#[test]
fn natives_by_their_class() {
    let vm = run("Streams");
    let counts = natives_by_class(&vm.natives);
    let (most, n) = counts.iter().max_by_key(|&(_, &n)| n).unwrap();
    assert_eq!((most.as_str(), *n), ("jdk/internal/misc/Unsafe", 11));
    assert_eq!(counts.len(), 13);
}
