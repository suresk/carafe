//! Tests for the Chapter 5 challenges: `cargo test --test challenges_ch05`.
use carafe::{Class, ClassFile, Jdk, Value, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
fn predict_the_counts() {
    let calls = Rc::new(Class::new(parse("Calls", "Calls")));
    let fib = |n: u64| -> u64 { (0..n).fold((0, 1), |(a, b), _| (b, a + b)).0 };
    for n in 0..=20 {
        let mut vm = Vm::new();
        vm.invoke(&calls, "fib", "(I)I", &[Value::Int(n as i32)]).unwrap();
        assert_eq!(vm.calls, 2 * fib(n + 1) - 2, "calls for {n}");
        assert_eq!(vm.executed, 19 * fib(n + 1) - 13, "instructions for {n}");
    }
}

// @challenge 2
use carafe::{call_graph, recursive};

#[test]
fn calls_and_recursion() {
    let calls = parse("Calls", "Calls");
    let graph = call_graph(&calls).unwrap();
    let callees = |m: &str| graph[m].iter().cloned().collect::<Vec<_>>();
    assert_eq!(callees("middle(J)J"), ["Calls.pick(IJI)J"]);
    assert_eq!(callees("abs(I)I"), ["java/lang/Math.abs(I)I"]);
    assert!(callees("nothing()V").is_empty());
    let found: Vec<String> = recursive(&calls).unwrap().into_iter().collect();
    assert_eq!(found, ["depth(I)I", "fib(I)I", "forever(I)I"]);
    assert!(recursive(&parse("Arith", "Arith")).unwrap().is_empty());
}
