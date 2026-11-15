//! Tests for the Chapter 4 challenges: `cargo test --test challenges_ch04`.
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

fn class(source: &str, name: &str) -> Rc<Class> {
    let bytes = std::fs::read(compile(source, name)).unwrap();
    Rc::new(Class::new(ClassFile::parse(&bytes).unwrap()))
}

// @challenge 1
#[test]
fn predict_the_count() {
    let arith = class("Arith", "Arith");
    for n in 0..50 {
        let mut vm = Vm::new();
        vm.invoke(&arith, "sum", "(I)I", &[Value::Int(n)]).unwrap();
        assert_eq!(vm.executed, 9 * n as u64 + 9);
    }
}

// @challenge 2
use carafe::call_checked;

#[test]
fn calls_are_checked() {
    let arith = class("Arith", "Arith");
    let mut vm = Vm::new();
    let ok = call_checked(
        &mut vm,
        &arith,
        "gcd",
        "(II)I",
        &[Value::Int(12), Value::Int(18)],
    );
    assert_eq!(ok.unwrap(), Some(Value::Int(6)));
    let few = call_checked(&mut vm, &arith, "gcd", "(II)I", &[Value::Int(12)]);
    assert!(few.is_err());
    let long = call_checked(&mut vm, &arith, "sum", "(I)I", &[Value::Long(3)]);
    assert!(long.is_err());
    let float = call_checked(&mut vm, &arith, "sum", "(I)I", &[Value::Float(3.0)]);
    assert!(float.is_err());
}

// @challenge 3
use carafe::max_stack;

#[test]
fn max_stack_matches_javac() {
    let mut checked = 0;
    for (source, name) in
        [("Arith", "Arith"), ("Switches", "Switches"), ("Constants", "Constants")]
    {
        let class = class(source, name);
        for method in &class.file.methods {
            let Some(code) = &method.code else { continue };
            match max_stack(&class, code) {
                Ok(depth) => {
                    assert_eq!(depth, usize::from(code.max_stack), "{}", method.name);
                    checked += 1;
                }
                Err(e) => assert!(e.to_string().contains("cannot run"), "{e}"),
            }
        }
    }
    assert_eq!(checked, 13);
}
