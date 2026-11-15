//! Tests for the Chapter 6 challenges: `cargo test --test challenges_ch06`.
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
fn predict_the_conversions() {
    let numbers = Rc::new(Class::new(parse("Numbers", "Numbers")));
    let call = |name: &str, descriptor: &str, arg: Value| {
        Vm::new().invoke(&numbers, name, descriptor, &[arg]).unwrap().unwrap()
    };
    assert_eq!(call("f2i", "(F)I", Value::Float(3e9)), Value::Int(i32::MAX));
    assert_eq!(call("d2l", "(D)J", Value::Double(-0.5)), Value::Long(0));
    assert_eq!(call("i2s", "(I)S", Value::Int(40_000)), Value::Int(-25_536));
    assert_eq!(call("l2f", "(J)F", Value::Long(i64::MAX)), Value::Float(9.223372e18));
    assert_eq!(call("d2f", "(D)F", Value::Double(0.1)), Value::Float(0.1));
    assert_eq!(
        call("f2d", "(F)D", Value::Float(0.1)),
        Value::Double(0.10000000149011612)
    );
}

// @challenge 2
use carafe::float_comparisons;

#[test]
fn how_javac_compares_floats() {
    let numbers = parse("Numbers", "Numbers");
    let pairs = float_comparisons(&numbers, "compareFloats").unwrap();
    let pairs: Vec<(&str, &str)> =
        pairs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    assert_eq!(
        pairs,
        [
            ("fcmpg", "ifge"), // a < b
            ("fcmpl", "ifle"), // a > b
            ("fcmpl", "ifne"), // a == b
            ("fcmpg", "ifgt"), // a <= b
            ("fcmpl", "iflt"), // a >= b
            ("fcmpl", "ifeq"), // a != b
        ]
    );
    assert_eq!(float_comparisons(&numbers, "compareDoubles").unwrap().len(), 6);
}

// @challenge 3
use carafe::f2i_by_hand;

#[test]
fn f2i_without_as() {
    for x in [0.0, -0.0, 0.5, -0.5, 1.0, -1.9, 2.5e9, -2.5e9, f32::NAN, f32::INFINITY]
    {
        assert_eq!(f2i_by_hand(x), x as i32, "{x}");
    }
    assert_eq!(f2i_by_hand(-2147483648.0), i32::MIN);
    // A sample of every bit pattern: about a million of them.
    for bits in (0..=u32::MAX).step_by(4099) {
        let x = f32::from_bits(bits);
        assert_eq!(f2i_by_hand(x), x as i32, "{bits:#x}");
    }
}
