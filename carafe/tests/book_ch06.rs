//! The examples in Chapter 6: `cargo test --test book_ch06`.
use carafe::{Class, ClassFile, Jdk, Value, Vm};
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
/// `Numbers`, loaded for running.
fn numbers() -> Rc<Class> {
    let bytes = std::fs::read(compile("Numbers", "Numbers")).unwrap();
    Rc::new(Class::new(ClassFile::parse(&bytes).unwrap()))
}

/// Calls the static method `name` of `Numbers` with one argument.
fn call(name: &str, descriptor: &str, arg: Value) -> Value {
    let result = Vm::new().invoke(&numbers(), name, descriptor, &[arg]).unwrap();
    result.expect("a result")
}
// @end

#[test]
fn conversions() {
    // @listing conversion-test
    assert_eq!(call("f2i", "(F)I", Value::Float(f32::NAN)), Value::Int(0));
    assert_eq!(call("f2i", "(F)I", Value::Float(1e10)), Value::Int(i32::MAX));
    assert_eq!(call("d2l", "(D)J", Value::Double(-2.9)), Value::Long(-2));
    assert_eq!(
        call("i2f", "(I)F", Value::Int(16_777_217)),
        Value::Float(16_777_216.0)
    );
    assert_eq!(call("l2i", "(J)I", Value::Long(1 << 40 | 7)), Value::Int(7));
    assert_eq!(call("i2c", "(I)C", Value::Int(-1)), Value::Int(65535));
    assert_eq!(call("i2b", "(I)B", Value::Int(300)), Value::Int(44));
    let Value::Float(f) = call("d2f", "(D)F", Value::Double(1e300)) else { panic!() };
    assert_eq!(f, f32::INFINITY);
    // @end
}

#[test]
fn nan_and_negative_zero() {
    // @listing nan-test
    let class = numbers();
    let compare = |a: f32, b: f32| {
        let args = [Value::Float(a), Value::Float(b)];
        Vm::new().invoke(&class, "compareFloats", "(FF)I", &args).unwrap()
    };
    // Bits: 1 <, 2 >, 4 ==, 8 <=, 16 >=, 32 !=.
    assert_eq!(compare(1.0, 2.0), Some(Value::Int(1 | 8 | 32)));
    assert_eq!(compare(f32::NAN, 2.0), Some(Value::Int(32))); // only !=
    assert_eq!(compare(f32::NAN, f32::NAN), Some(Value::Int(32)));
    assert_eq!(compare(-0.0, 0.0), Some(Value::Int(4 | 8 | 16))); // equal

    let args = [Value::Float(0.0), Value::Float(0.0), Value::Int(5)]; // -a
    let negated = Vm::new().invoke(&class, "floats", "(FFI)F", &args).unwrap();
    let Some(Value::Float(z)) = negated else { panic!() };
    assert_eq!(z.to_bits(), 0x8000_0000); // -0.0, not 0.0
    // @end
}

#[test]
fn long_division_by_zero() {
    // @listing long-test
    let class = numbers();
    let args = [Value::Long(i64::MIN), Value::Long(-1), Value::Int(3)]; // a / b
    let quotient = Vm::new().invoke(&class, "longs", "(JJI)J", &args).unwrap();
    assert_eq!(quotient, Some(Value::Long(i64::MIN)));
    let args = [Value::Long(1), Value::Long(0), Value::Int(4)]; // a % b
    let error = Vm::new().invoke(&class, "longs", "(JJI)J", &args).unwrap_err();
    assert_eq!(error.to_string(), "java.lang.ArithmeticException: / by zero");
    // @end
}

// @listing parse-value
/// A value as `Numbers.main` prints it: a type letter and a number, which is
/// the raw bits of a `float` or `double`.
fn value(text: &str) -> Value {
    let (ty, number) = text.split_once(':').unwrap();
    match ty {
        "J" => Value::Long(number.parse().unwrap()),
        "F" => Value::Float(f32::from_bits(number.parse::<i32>().unwrap() as u32)),
        "D" => Value::Double(f64::from_bits(number.parse::<i64>().unwrap() as u64)),
        _ => Value::Int(number.parse().unwrap()),
    }
}

/// Whether two results are the same: bit for bit, except that any NaN
/// equals any other, since the JVMS doesn't fix a NaN's bits.
fn same(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => {
            x.to_bits() == y.to_bits() || x.is_nan() && y.is_nan()
        }
        (Value::Double(x), Value::Double(y)) => {
            x.to_bits() == y.to_bits() || x.is_nan() && y.is_nan()
        }
        _ => a == b,
    }
}
// @end

#[test]
fn every_result_matches_hotspot() {
    // @listing hotspot-test
    let class = numbers();
    let classes = compile("Numbers", "Numbers").parent().unwrap().to_path_buf();
    let printed = Jdk::find().unwrap().java(&classes, "Numbers").unwrap();
    let mut checked = 0;
    for line in printed.lines() {
        // longs (JJI)J J:5 J:3 I:0 = J:8
        let words: Vec<&str> = line.split_whitespace().collect();
        let [name, descriptor, args @ .., "=", result] = &words[..] else {
            panic!("{line}")
        };
        let args: Vec<Value> = args.iter().map(|a| value(a)).collect();
        let ours = Vm::new().invoke(&class, name, descriptor, &args).unwrap();
        assert!(same(ours.unwrap(), value(result)), "{line}: carafe gives {ours:?}");
        checked += 1;
    }
    assert_eq!(checked, 3376);
    // @end
}
