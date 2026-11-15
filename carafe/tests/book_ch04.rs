//! The examples in Chapter 4: `cargo test --test book_ch04`.
use carafe::{Class, ClassFile, Frame, Jdk, Value, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Compiles `java/<source>.java` with the pinned JDK and returns the path of
/// the class file for `class`, one of the classes the source declares.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

// @listing helpers
/// Loads the class `name`, compiled from `java/<source>.java`, for running.
fn class(source: &str, name: &str) -> Rc<Class> {
    let bytes = std::fs::read(compile(source, name)).unwrap();
    Rc::new(Class::new(ClassFile::parse(&bytes).unwrap()))
}

/// Calls the static method `name` of `class`, which takes and returns `int`s,
/// and returns its result and the number of instructions it executed.
fn call(class: &Rc<Class>, name: &str, args: &[i32]) -> (i32, u64) {
    let descriptor = format!("({})I", "I".repeat(args.len()));
    let args: Vec<Value> = args.iter().map(|&a| Value::Int(a)).collect();
    let mut vm = Vm::new();
    match vm.invoke(class, name, &descriptor, &args).unwrap() {
        Some(Value::Int(result)) => (result, vm.executed),
        other => panic!("{name} returned {other:?}"),
    }
}
// @end

#[test]
fn a_frame() {
    // @listing frame-test
    let arith = class("Arith", "Arith");
    let sum = arith.method("sum", "(I)I").unwrap();
    let mut frame = Frame::new(arith.clone(), sum, &[Value::Int(10)]).unwrap();
    assert_eq!(frame.locals, [Value::Int(10), Value::Top, Value::Top]);
    assert_eq!(std::mem::size_of::<Value>(), 16);

    frame.push(Value::Long(7)).unwrap();
    assert_eq!(frame.stack, [Value::Long(7), Value::Top]); // two slots
    let full = frame.push(Value::Int(1)).unwrap_err().to_string();
    assert_eq!(full, "VerifyError: sum overflows its max_stack"); // max_stack is 2
    assert_eq!(frame.pop().unwrap(), Value::Long(7));
    assert!(frame.pop().is_err());

    let wrong = Frame::new(arith.clone(), sum, &[Value::Long(10)]).err().unwrap();
    assert_eq!(
        wrong.to_string(),
        "VerifyError: sum was given 2 slots of arguments, not 1"
    );
    // @end
}

#[test]
fn a_loop() {
    // @listing sum-test
    let arith = class("Arith", "Arith");
    assert_eq!(call(&arith, "sum", &[100]), (5050, 909));
    assert_eq!(call(&arith, "sum", &[0]), (0, 9));
    // @end
}

#[test]
fn more_loops() {
    // @listing loops-test
    let arith = class("Arith", "Arith");
    assert_eq!(call(&arith, "gcd", &[1071, 462]), (21, 37));
    assert_eq!(call(&arith, "collatz", &[27]), (111, 1602));
    assert_eq!(call(&arith, "bits", &[-1]), (32, 422));
    assert_eq!(call(&arith, "bits", &[0b1011]), (3, 58));
    // @end
}

#[test]
fn java_arithmetic() {
    // @listing arithmetic-test
    let arith = class("Arith", "Arith");
    assert_eq!(call(&arith, "next", &[i32::MAX]).0, i32::MIN); // wraps
    assert_eq!(call(&arith, "divide", &[-7, 2]).0, -3); // rounds toward zero
    assert_eq!(call(&arith, "divide", &[i32::MIN, -1]).0, i32::MIN);

    let mut vm = Vm::new();
    let args = [Value::Int(1), Value::Int(0)];
    let error = vm.invoke(&arith, "divide", "(II)I", &args).unwrap_err();
    assert_eq!(error.to_string(), "java.lang.ArithmeticException: / by zero");
    // @end
}

#[test]
fn switches() {
    // @listing switch-test
    let switches = class("Switches", "Switches");
    let days: Vec<i32> = (1..=12).map(|m| call(&switches, "days", &[m]).0).collect();
    assert_eq!(days, [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]);

    let mut vm = Vm::new();
    let error =
        vm.invoke(&switches, "size", "(I)Ljava/lang/String;", &[Value::Int(10)]);
    assert_eq!(
        error.unwrap_err().to_string(),
        "carafe cannot run this yet: ldc of String { utf8: 10 }"
    );
    // @end
}
