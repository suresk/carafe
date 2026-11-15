//! Tests for the Chapter 23 challenges: `cargo test --test challenges_ch23`.
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

/// Runs `Bench` with this input and returns the VM.
fn bench(input: &str) -> Vm {
    let app = compile("Bench", "Bench").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.stdin = input.as_bytes().to_vec();
    vm.run_main("Bench").unwrap();
    vm
}

// @challenge 1
#[test]
fn predict_the_calls() {
    // fib(n) calls itself 2 * fib(n + 1) - 2 times, so fib(20) makes
    // 2 * (fib(21) - fib(20)) = 2 * fib(19) more calls than fib(19).
    let calls = bench("fib 20\n").calls - bench("fib 19\n").calls;
    assert_eq!(calls, 2 * 4181);
}

// @challenge 2
use carafe::histogram;

#[test]
fn a_histogram() {
    let mut vm = bench("none 0\n");
    let class = vm.load("Bench").unwrap();
    let counts = histogram(&class);
    let mut top: Vec<(&String, &usize)> = counts.iter().collect();
    top.sort_by_key(|&(name, &n)| (std::cmp::Reverse(n), name.clone()));
    let names: Vec<&str> =
        top.iter().take(3).map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["iload", "goto", "invokevirtual"]);
}

// @challenge 3
use carafe::cache_census;

#[test]
fn what_the_cache_holds() {
    let mut vm = bench("");
    let class = vm.load("Bench").unwrap();
    let census = cache_census(&class);
    let census: Vec<(&str, usize)> = census.into_iter().collect();
    assert_eq!(census, [("class", 7), ("field", 2), ("method", 26), ("static", 2)]);
}
