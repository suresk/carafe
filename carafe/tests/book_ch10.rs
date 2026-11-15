//! The examples in Chapter 10: `cargo test --test book_ch10`.
use carafe::{ARRAY, FieldType, Jdk, LATIN1, UTF16, Value, Vm};
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

// @listing helpers
/// A VM on the JDK's class library, with the program `source` compiled and
/// on its class path.
fn boot(source: &str) -> Vm {
    let app = compile(source, source).parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap()
}

/// The bytes of the array at `array`, read through the heap.
fn bytes(vm: &Vm, array: carafe::Ref) -> Vec<u8> {
    let length = vm.heap.array_length(array) as i32;
    let at = |i| vm.heap.element(array, i).unwrap();
    let byte = |i| match vm.heap.read(at(i), &FieldType::Byte) {
        Value::Int(b) => b as u8,
        other => panic!("{other:?}"),
    };
    (0..length).map(byte).collect()
}
// @end

#[test]
fn strings() {
    // @listing strings-test
    let mut vm = boot("Hello");
    let field = |vm: &Vm, s: carafe::Ref, name: &str, ty: &str| {
        let class = vm.heap.class_of(s).clone();
        let slot = class.field(name, ty).unwrap();
        vm.heap.read(s.0 + slot.offset, &slot.ty)
    };
    let units: Vec<u16> = "café".encode_utf16().collect();
    let cafe = vm.new_string(&units).unwrap();
    assert_eq!(field(&vm, cafe, "coder", "B"), Value::Int(LATIN1));
    let Value::Ref(value) = field(&vm, cafe, "value", "[B") else { panic!() };
    assert_eq!(bytes(&vm, value), [b'c', b'a', b'f', 0xe9]); // é is 233

    let units: Vec<u16> = "a€".encode_utf16().collect();
    let euro = vm.new_string(&units).unwrap();
    assert_eq!(field(&vm, euro, "coder", "B"), Value::Int(UTF16));
    let Value::Ref(value) = field(&vm, euro, "value", "[B") else { panic!() };
    assert_eq!(bytes(&vm, value), [0x61, 0, 0xac, 0x20]); // U+0061, U+20AC
    assert_eq!(vm.rust_string(euro).unwrap(), "a€");

    let one = vm.intern(&units).unwrap();
    assert_eq!(vm.intern(&units).unwrap(), one); // the same object
    assert_ne!(one, euro); // new_string doesn't intern
    // @end
}

#[test]
fn an_array_header() {
    // @listing array-test
    let mut vm = boot("Hello");
    let array = vm.heap.new_array(&FieldType::Int, 3).unwrap();
    let at = vm.heap.element(array, 2).unwrap();
    vm.heap.write(at, &FieldType::Int, Value::Int(-1)).unwrap();
    let start = array.0 as usize;
    #[rustfmt::skip]
    assert_eq!(
        vm.heap.bytes()[start..start + 32],
        [
            // ARRAY | element type 0, hash 8, length 3, 4 unused bytes
            0, 0, 0, 0x80,  8, 0, 0, 0,   3, 0, 0, 0,  0, 0, 0, 0,
            // elements 0 and 1, then 2 = -1, then padding to 8
            0, 0, 0, 0,     0, 0, 0, 0,   0xff, 0xff, 0xff, 0xff,  0, 0, 0, 0,
        ]
    );
    assert_eq!(ARRAY, 0x8000_0000);
    let error = vm.heap.element(array, 3).unwrap_err().to_string();
    assert_eq!(
        error,
        "java.lang.ArrayIndexOutOfBoundsException: Index 3 out of bounds for length 3"
    );
    // @end
}

#[test]
fn hello_world() {
    // @listing hello-test
    let mut vm = boot("Hello");
    let hello = vm.load("Hello").unwrap();
    let args =
        vm.heap.new_array(&FieldType::Object("java/lang/String".into()), 0).unwrap();
    vm.invoke(&hello, "main", "([Ljava/lang/String;)V", &[Value::Ref(args)]).unwrap();
    assert_eq!(String::from_utf8(vm.stdout.clone()).unwrap(), "Hello, world!\n");
    assert_eq!(
        vm.loaded,
        [
            "java/lang/Object",
            "Hello",
            "java/lang/System",
            "java/io/PrintStream",
            "java/io/Serializable",
            "java/lang/Comparable",
            "java/lang/CharSequence",
            "java/lang/constant/Constable",
            "java/lang/constant/ConstantDesc",
            "java/lang/String",
        ]
    );
    assert_eq!(vm.natives.len(), 1); // PrintStream.write0
    assert_eq!((vm.executed, vm.calls), (53, 11));
    // @end
}

#[test]
fn the_same_as_hotspot() {
    // @listing hotspot-test
    for program in ["Hello", "Greeter"] {
        let mut vm = boot(program);
        vm.run_main(program).unwrap();
        let classes = compile(program, program).parent().unwrap().to_path_buf();
        let jdk = Jdk::find().unwrap();
        let utf8 = ["-Dstdout.encoding=UTF-8"];
        let hotspot = jdk.java_with(&utf8, &classes, program).unwrap();
        assert_eq!(String::from_utf8(vm.stdout).unwrap(), hotspot);
    }
    // @end
}

#[test]
fn a_missing_native() {
    // @listing native-test
    let mut vm = boot("Greeter");
    let greeter = vm.load("Greeter").unwrap();
    let error = vm.invoke(&greeter, "now", "()J", &[]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "java.lang.UnsatisfiedLinkError: 'long Greeter.ticks()'"
    );
    // @end
}
