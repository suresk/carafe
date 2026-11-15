//! The examples in Chapter 21: `cargo test --test book_ch21`.
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

/// Runs `program` on carafe and on HotSpot, and returns carafe's VM, what it
/// printed, and what HotSpot printed.
fn both(program: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java(&app, program).unwrap();
    (vm, ours, theirs)
}

#[test]
fn pipelines() {
    // @listing pipelines-test
    let (_, ours, theirs) = both("Pipelines");
    assert_eq!(ours, theirs);
    assert!(ours.contains("{France=[Lyon, Paris], Italy=[Turin, Milan, Rome], "));
    // @end
}

#[test]
fn stress() {
    // @listing stress-test
    let (vm, ours, theirs) = both("Stress");
    assert_eq!(ours, theirs);
    assert!(vm.executed > 15_000_000); // sorting and grouping 10,000 numbers
    // @end
}

#[test]
fn the_cost_of_a_stream() {
    // @listing cost-test
    let (hello, _, _) = both("Hello");
    let (pipelines, _, _) = both("Pipelines");
    let stream = |vm: &Vm| {
        vm.loaded.iter().filter(|c| c.starts_with("java/util/stream/")).count()
    };
    assert_eq!(stream(&pipelines), 101);
    assert_eq!(pipelines.loaded.len() - hello.loaded.len(), 309);
    assert_eq!(pipelines.executed - hello.executed, 118_758);
    let new: Vec<&String> = pipelines.natives.difference(&hello.natives).collect();
    assert_eq!(new.len(), 32);
    // @end
}
