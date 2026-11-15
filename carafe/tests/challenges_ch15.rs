//! Tests for the Chapter 15 challenges: `cargo test --test challenges_ch15`.
use carafe::{Jdk, Value, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn run(program: &str) -> Vm {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main(program).unwrap();
    vm
}

// @challenge 1
#[test]
fn predict_the_hashes() {
    let vm = run("Collections15");
    let output = String::from_utf8(vm.stdout).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    // Objects.hash(1, "a", 'c'): 31 * (31 * (31 * 1 + 1) + 97) + 99.
    assert_eq!(lines[16], (31 * (31 * (31 + 1) + 97) + 99).to_string());
    // A HashSet of 16 buckets orders Characters by code % 16: p 0, s 3, i 9, m 13.
    let order: Vec<char> = "psim".chars().collect();
    assert!(order.windows(2).all(|w| (w[0] as u32 % 16) < (w[1] as u32 % 16)));
    assert_eq!(lines[8], "[p, s, i, m]");
}

// @challenge 2
use carafe::get_interfaces;

#[test]
fn the_interfaces_of_a_class() {
    let mut vm = run("Hello");
    let list = vm.load("java/util/ArrayList").unwrap();
    let mirror = vm.mirror(&list).unwrap();
    let Some(Value::Ref(array)) =
        get_interfaces(&mut vm, &[Value::Ref(mirror)]).unwrap()
    else {
        panic!()
    };
    let class_type = carafe::FieldType::Object("java/lang/Class".into());
    let names: Vec<String> = (0..vm.heap.array_length(array) as i32)
        .map(|i| {
            let Value::Ref(m) =
                vm.heap.read(vm.heap.element(array, i).unwrap(), &class_type)
            else {
                panic!()
            };
            vm.class_of_mirror(m).unwrap().file.this_class.clone()
        })
        .collect();
    assert_eq!(
        names,
        [
            "java/util/List",
            "java/util/RandomAccess",
            "java/lang/Cloneable",
            "java/io/Serializable"
        ]
    );
}

// @challenge 3
use carafe::census;

#[test]
fn a_heap_census() {
    let mut vm = run("Collections15");
    let counts = census(&mut vm).unwrap();
    assert_eq!(counts["java/util/HashMap$Node"], 16); // two maps of six words, a set of four
    assert_eq!(counts["java/util/TreeMap$Entry"], 6);
    assert!(counts["java/lang/String"] > 30);
}
