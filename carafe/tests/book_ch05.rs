//! The examples in Chapter 5: `cargo test --test book_ch05`.
use carafe::{Class, ClassFile, Jdk, MAX_FRAMES, Value, Vm};
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
/// The class file of `Calls`, as bytes.
fn calls_bytes() -> Vec<u8> {
    std::fs::read(compile("Calls", "Calls")).unwrap()
}

/// A class, for running, from the bytes of its class file.
fn class(bytes: &[u8]) -> Rc<Class> {
    Rc::new(Class::new(ClassFile::parse(bytes).unwrap()))
}
// @end

#[test]
fn fibonacci() {
    // @listing fib-test
    let calls = class(&calls_bytes());
    let mut vm = Vm::new();
    let fib = vm.invoke(&calls, "fib", "(I)I", &[Value::Int(20)]).unwrap();
    assert_eq!(fib, Some(Value::Int(6765)));
    assert_eq!(vm.calls, 21_890); // every call but the first, from Rust
    assert_eq!(vm.deepest, 20); // fib(20) down to fib(1)
    assert_eq!(vm.executed, 207_961);
    // @end
}

#[test]
fn deep_recursion() {
    // @listing depth-test
    let calls = class(&calls_bytes());
    let mut vm = Vm::new();
    let depth = vm.invoke(&calls, "depth", "(I)I", &[Value::Int(5000)]).unwrap();
    assert_eq!((depth, vm.deepest), (Some(Value::Int(5000)), 5001));

    let mut vm = Vm::new();
    let error = vm.invoke(&calls, "forever", "(I)I", &[Value::Int(0)]).unwrap_err();
    assert_eq!(error.to_string(), "java.lang.StackOverflowError");
    assert_eq!(vm.deepest, MAX_FRAMES);
    // The VM is usable again: invoke unwound the frames.
    let again = vm.invoke(&calls, "depth", "(I)I", &[Value::Int(3)]).unwrap();
    assert_eq!(again, Some(Value::Int(3)));
    // @end
}

#[test]
fn two_slot_arguments() {
    // @listing slots-test
    let calls = class(&calls_bytes());
    let mut vm = Vm::new();
    let big = Value::Long(1 << 40);
    assert_eq!(vm.invoke(&calls, "middle", "(J)J", &[big]).unwrap(), Some(big));
    let args = [Value::Double(0.5), Value::Double(2.5)];
    let second = vm.invoke(&calls, "second", "(DD)D", &args).unwrap();
    assert_eq!(second, Some(Value::Double(2.5)));
    assert_eq!(vm.invoke(&calls, "nothing", "()V", &[]).unwrap(), None);
    // @end
}

#[test]
fn ireturn_narrows() {
    // @listing narrow-test
    // low(int x) is `iload_0, i2b, ireturn`: turn the i2b into a nop, so
    // that ireturn's narrowing is all that makes the result a byte.
    let mut bytes = calls_bytes();
    let at = bytes.windows(3).position(|w| w == [0x1a, 0x91, 0xac]).unwrap();
    bytes[at + 1] = 0x00;
    let calls = class(&bytes);
    let mut vm = Vm::new();
    let low = |vm: &mut Vm, x| vm.invoke(&calls, "low", "(I)B", &[Value::Int(x)]);
    assert_eq!(low(&mut vm, 300).unwrap(), Some(Value::Int(44)));
    assert_eq!(low(&mut vm, 255).unwrap(), Some(Value::Int(-1)));
    // @end
}
