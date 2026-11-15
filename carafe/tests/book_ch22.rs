//! The examples in Chapter 22: `cargo test --test book_ch22`.
use carafe::{Jdk, Vm};
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

// @listing input-helper
/// Runs `program` on carafe and on HotSpot with `input` as standard input,
/// and returns carafe's VM, what it printed, and what HotSpot printed.
fn both_with(program: &str, input: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.stdin = input.as_bytes().to_vec();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java_input(&app, program, input.as_bytes()).unwrap();
    (vm, ours, theirs)
}
// @end

#[test]
fn reading_input() {
    // @listing input-test
    let input = "12 apples\n-3 café 40\n\n  naïve 1000000000000\n";
    let (_, ours, theirs) = both_with("Input", input);
    assert_eq!(ours, theirs);
    assert!(ours.contains("CAFÉ 4\n")); // two bytes of UTF-8, one char
    assert!(ours.ends_with("4 lines, total 1000000000049\n-1\n"));
    // @end
}

#[test]
fn the_main_thread() {
    // @listing thread-test
    let (vm, ours, _) = both_with("Input", "");
    assert!(ours.starts_with("main in main\nRUNNABLE true 5 false\n"));
    assert!(
        vm.natives.contains("java/lang/Thread.currentThread()Ljava/lang/Thread;")
    );
    let (hello, _, _) = both_with("Hello", "");
    assert!(!hello.natives.iter().any(|n| n.starts_with("java/lang/Thread.")));
    // @end
}
