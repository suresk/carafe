//! Tests for the Chapter 22 challenges: `cargo test --test challenges_ch22`.
use carafe::{Error, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn run(program: &str) -> Vm {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main(program).unwrap();
    vm
}

// @challenge 1
#[test]
fn who_needs_a_thread() {
    let made = |program: &str| run(program).thread.is_some();
    // Reading input locks, and a lock records the thread that holds it.
    assert_eq!(
        [made("Hello"), made("Pipelines"), made("Input")],
        [false, false, true]
    );
}

// @challenge 2
use carafe::thread_summary;

#[test]
fn the_thread_in_the_heap() {
    let mut vm = run("Input");
    let summary = thread_summary(&mut vm).unwrap();
    assert_eq!(summary, Some(("main".to_string(), "main".to_string(), 5)));
    assert_eq!(thread_summary(&mut run("Hello")).unwrap(), None);
}

// @challenge 3
#[test]
fn a_scanner() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("scanner");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("Scan.java");
    let text = "public class Scan { public static void main(String[] a) { \
                System.out.println(new java.util.Scanner(System.in).nextInt()); } }";
    std::fs::write(&source, text).unwrap();
    let jdk = Jdk::find().unwrap();
    jdk.javac(&source, &dir).unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
    vm.stdin = b"42\n".to_vec();
    let Err(Error::Thrown { class, .. }) = vm.run_main("Scan") else { panic!() };
    assert_eq!(class, "java/util/ServiceConfigurationError");
}
