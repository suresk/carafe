//! The examples in Chapter 23: `cargo test --test book_ch23`.
use carafe::{Jdk, Resolved, Vm};
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

#[test]
fn the_benchmarks() {
    // @listing bench-test
    let app = compile("Bench", "Bench").parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main("Bench").unwrap(); // no input: each benchmark, small
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    assert_eq!(ours, jdk.java_input(&app, "Bench", b"").unwrap());
    let bench = vm.load("Bench").unwrap();
    let count = bench.file.constant_pool.count();
    let cached: Vec<&Resolved> = (1..count).filter_map(|i| bench.cached(i)).collect();
    let kinds = [
        cached.iter().filter(|r| matches!(r, Resolved::Class(_))).count(),
        cached.iter().filter(|r| matches!(r, Resolved::Method(..))).count(),
        cached.iter().filter(|r| matches!(r, Resolved::Field(_))).count(),
        cached.iter().filter(|r| matches!(r, Resolved::Static(..))).count(),
    ];
    assert!(kinds.iter().all(|&n| n > 0), "{kinds:?}");
    // @end
}
