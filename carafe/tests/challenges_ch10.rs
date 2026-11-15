//! Tests for the Chapter 10 challenges: `cargo test --test challenges_ch10`.
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

fn boot(source: &str) -> Vm {
    let app = compile(source, source).parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap()
}

// @challenge 1
#[test]
fn predict_the_bytes() {
    let mut vm = boot("Hello");
    vm.load("java/lang/String").unwrap();
    let before = vm.heap.bytes().len();
    let units: Vec<u16> = "Hi".encode_utf16().collect();
    vm.new_string(&units).unwrap();
    assert_eq!(vm.heap.bytes().len() - before, 24 + 24); // the String, then its byte[2]
    let before = vm.heap.bytes().len();
    let units: Vec<u16> = "😀".encode_utf16().collect();
    vm.new_string(&units).unwrap();
    assert_eq!(vm.heap.bytes().len() - before, 24 + 24); // 4 bytes still fit in 24
    let string = vm.load("java/lang/String").unwrap();
    assert_eq!(string.layout.size, 18);
}

// @challenge 2
use carafe::initialized;

#[test]
fn what_hello_initializes() {
    let mut vm = boot("Hello");
    vm.run_main("Hello").unwrap();
    let names = initialized(&mut vm).unwrap();
    assert_eq!(
        names,
        ["java/lang/Object", "Hello", "java/lang/System", "java/io/PrintStream"]
    );
}

// @challenge 3
use carafe::java_hash;

#[test]
fn hash_codes_match_hotspot() {
    let mut vm = boot("HashCodes");
    let classes = compile("HashCodes", "HashCodes").parent().unwrap().to_path_buf();
    let printed = Jdk::find().unwrap().java(&classes, "HashCodes").unwrap();
    let samples =
        ["", "a", "Hello, world!", "café", "a€", "😀", "polygenelubricants"];
    for (text, line) in samples.iter().zip(printed.lines()) {
        let units: Vec<u16> = text.encode_utf16().collect();
        let string = vm.new_string(&units).unwrap();
        assert_eq!(java_hash(&vm, string).unwrap().to_string(), line, "{text}");
    }
    assert_eq!(printed.lines().count(), samples.len());
}
