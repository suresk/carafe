//! Tests for the Chapter 14 challenges: `cargo test --test challenges_ch14`.
use carafe::{FieldType, Jdk, Ref, Value, Vm, size_of};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn boot() -> Vm {
    let app = compile("Hello", "Hello").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap()
}

// @challenge 1
#[test]
fn predict_the_offsets() {
    let mut vm = boot();
    let atomic = vm.load("java/util/concurrent/atomic/AtomicInteger").unwrap();
    let value = atomic.field("value", "I").unwrap();
    assert_eq!(value.offset, 8); // objectFieldOffset(AtomicInteger.class, "value")
    assert_eq!(size_of(&FieldType::Long), 8); // arrayIndexScale(long[].class)
    let object = FieldType::Object("java/lang/Object".into());
    assert_eq!(size_of(&object), 4); // arrayIndexScale(Object[].class)
    assert_eq!(carafe::ARRAY_HEADER, 16); // arrayBaseOffset of every array class
}

// @challenge 2
use carafe::identity_hash_code;

#[test]
fn identity_hash_codes() {
    let mut vm = boot();
    let units: Vec<u16> = "x".encode_utf16().collect();
    let string = vm.new_string(&units).unwrap();
    let hash = identity_hash_code(&mut vm, &[Value::Ref(string)]).unwrap();
    assert_eq!(hash, Some(Value::Int(string.0 as i32)));
    let null = identity_hash_code(&mut vm, &[Value::Ref(Ref::NULL)]).unwrap();
    assert_eq!(null, Some(Value::Int(0)));
}

// @challenge 3
use carafe::native_coverage;

#[test]
fn how_much_of_unsafe() {
    let mut vm = boot();
    let (declared, done) =
        native_coverage(&mut vm, "jdk/internal/misc/Unsafe").unwrap();
    assert_eq!(declared, 68);
    assert_eq!(done, 45); // 44, and Chapter 21's objectFieldOffset0
    assert_eq!(native_coverage(&mut vm, "java/lang/Float").unwrap(), (2, 2));
}
