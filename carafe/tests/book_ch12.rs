//! The examples in Chapter 12: `cargo test --test book_ch12`.
use carafe::{FieldType, Jdk, Ref, Value, Vm};
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
/// A VM booted with `Sorting.java`'s classes, and what HotSpot prints for it.
fn sorting() -> (Vm, Vec<String>) {
    let app = compile("Sorting", "Sorting").parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let vm = Vm::boot(&jdk, &work, &app).unwrap();
    let utf8 = ["-Dstdout.encoding=UTF-8"];
    let printed = jdk.java_with(&utf8, &app, "Sorting").unwrap();
    (vm, printed.lines().map(String::from).collect())
}

/// Calls a static method of `Sorting`.
fn call(
    vm: &mut Vm,
    name: &str,
    descriptor: &str,
    args: &[Value],
) -> carafe::Result<Value> {
    let sorting = vm.load("Sorting")?;
    Ok(vm.invoke(&sorting, name, descriptor, args)?.unwrap_or(Value::Top))
}

/// An array as `Arrays.toString` and `Arrays.deepToString` print it.
fn show(vm: &Vm, value: Value) -> String {
    let Value::Ref(array) = value else { panic!("not a reference: {value:?}") };
    let ty = vm.heap.element_type(array).clone();
    let elements: Vec<String> = (0..vm.heap.array_length(array) as i32)
        .map(|i| {
            let element = vm.heap.read(vm.heap.element(array, i).unwrap(), &ty);
            match (&ty, element) {
                (FieldType::Char, Value::Int(c)) => {
                    char::from_u32(c as u32).unwrap().into()
                }
                (FieldType::Array(_), element) => show(vm, element),
                (_, Value::Int(v)) => v.to_string(),
                (_, Value::Long(v)) => v.to_string(),
                (_, other) => format!("{other:?}"),
            }
        })
        .collect();
    format!("[{}]", elements.join(", "))
}
// @end

#[test]
fn a_sort_written_in_java() {
    // @listing sort-test
    let (mut vm, hotspot) = sorting();
    let numbers = call(&mut vm, "numbers", "()[I", &[]).unwrap();
    assert_eq!(show(&vm, numbers), "[5, 3, 9, 1, 7, 2, 8, 6, 4, 0]");
    let sorted = call(&mut vm, "insertionSort", "([I)[I", &[numbers]).unwrap();
    assert_eq!(sorted, numbers); // sorted in place
    assert_eq!(show(&vm, sorted), hotspot[0]);
    // @end
}

#[test]
fn more_arrays() {
    // @listing arrays-test
    let (mut vm, hotspot) = sorting();
    let grid =
        call(&mut vm, "grid", "(II)[[J", &[Value::Int(2), Value::Int(3)]).unwrap();
    assert_eq!(show(&vm, grid), hotspot[1]); // [[0, 1, 2], [3, 4, 5]]

    let numbers = call(&mut vm, "numbers", "()[I", &[]).unwrap();
    let copy = call(&mut vm, "cloned", "([I)Ljava/lang/Object;", &[numbers]).unwrap();
    assert_ne!(copy, numbers); // a new array, with the same elements
    assert_eq!(show(&vm, copy), hotspot[2]);
    let shifted = call(&mut vm, "shifted", "([I)[I", &[numbers]).unwrap();
    assert_eq!(show(&vm, shifted), hotspot[3]); // overlapping ranges

    let units: Vec<u16> = "héllo".encode_utf16().collect();
    let text = Value::Ref(vm.intern(&units).unwrap());
    let letters =
        call(&mut vm, "letters", "(Ljava/lang/String;)[C", &[text]).unwrap();
    assert_eq!(show(&vm, letters), hotspot[4]);
    // @end
}

#[test]
fn array_classes() {
    // @listing classes-test
    let (mut vm, _) = sorting();
    let grid =
        call(&mut vm, "grid", "(II)[[J", &[Value::Int(2), Value::Int(3)]).unwrap();
    let Value::Ref(grid) = grid else { panic!() };
    let class = vm.class_of(grid).unwrap();
    assert_eq!(class.file.this_class, "[[J");
    assert_eq!(
        class.superclass.as_ref().unwrap().file.this_class,
        "java/lang/Object"
    );
    let interfaces: Vec<&str> =
        class.interfaces.iter().map(|i| i.file.this_class.as_str()).collect();
    assert_eq!(interfaces, ["java/lang/Cloneable", "java/io/Serializable"]);
    let at = |name: &str| vm.loaded.iter().position(|n| n == name).unwrap();
    assert!(at("[J") < at("[[J")); // the component class first
    // @end
}

#[test]
fn the_same_errors_as_hotspot() {
    // @listing errors-test
    let (mut vm, hotspot) = sorting();
    let mut errors = Vec::new();
    let numbers = call(&mut vm, "numbers", "()[I", &[]).unwrap();
    let failures: [(&str, &str, Vec<Value>); 6] = [
        ("copyTooFar", "([I)V", vec![numbers]),
        ("copyBackwards", "([I)V", vec![numbers]),
        ("copyNames", "()V", vec![]),
        ("copyMismatch", "()V", vec![]),
        ("pastTheEnd", "([I)I", vec![numbers]),
        ("negative", "(I)[I", vec![Value::Int(-1)]),
    ];
    for (name, descriptor, args) in failures {
        errors.push(call(&mut vm, name, descriptor, &args).unwrap_err().to_string());
    }
    assert_eq!(errors, hotspot[5..]);
    // @end
}

#[test]
fn null_arrays() {
    // @listing null-test
    let (mut vm, _) = sorting();
    let null = Value::Ref(Ref::NULL);
    let error = call(&mut vm, "pastTheEnd", "([I)I", &[null]).unwrap_err();
    assert_eq!(error.to_string(), "java.lang.NullPointerException");
    // @end
}
