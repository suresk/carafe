//! The examples in Chapter 13: `cargo test --test book_ch13`.
use carafe::{Jdk, Vm};
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
/// A VM booted with `Casts.java`'s classes, and the lines HotSpot prints.
fn casts() -> (Vm, Vec<String>) {
    let app = compile("Casts", "Casts").parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let vm = Vm::boot(&jdk, &work, &app).unwrap();
    let printed = jdk.java(&app, "Casts").unwrap();
    (vm, printed.lines().map(String::from).collect())
}

/// Runs the static method `name` of `Casts`, which takes no arguments.
fn run(vm: &mut Vm, name: &str) -> carafe::Result<()> {
    let casts = vm.load("Casts")?;
    vm.invoke(&casts, name, "()V", &[]).map(|_| ())
}
// @end

#[test]
fn the_same_as_hotspot() {
    // @listing hotspot-test
    let (mut vm, hotspot) = casts();
    for name in ["interfaces", "casts", "list"] {
        run(&mut vm, name).unwrap();
    }
    let output = String::from_utf8(vm.stdout.clone()).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines, hotspot[..lines.len()]);
    assert_eq!(lines[..5], ["robot", "Hello", "person", "Hi there", "left"]);

    let mut errors = Vec::new();
    for name in ["badCast", "badArrayCast", "badStore", "badCopy"] {
        errors.push(run(&mut vm, name).unwrap_err().to_string());
    }
    assert_eq!(errors, hotspot[lines.len()..]);
    // @end
}

#[test]
fn assignability() {
    // @listing assignable-test
    let (mut vm, _) = casts();
    let mut assignable = |s: &str, t: &str| {
        let (s, t) = (vm.load(s).unwrap(), vm.load(t).unwrap());
        vm.is_assignable(&s, &t).unwrap()
    };
    assert!(assignable("java/lang/String", "java/lang/Comparable"));
    assert!(assignable("Named", "java/lang/Object")); // an interface
    assert!(assignable("[I", "java/lang/Cloneable"));
    assert!(!assignable("[I", "[J"));
    assert!(assignable("[Ljava/lang/String;", "[Ljava/lang/Object;"));
    assert!(assignable("[LRobot;", "[LNamed;"));
    assert!(!assignable("[LNamed;", "[LRobot;"));
    assert!(assignable("[[I", "[Ljava/lang/Object;")); // an int[] is an Object
    // @end
}

#[test]
fn default_methods() {
    // @listing default-test
    let (mut vm, _) = casts();
    let named = vm.load("Named").unwrap();
    let greeting = named.method("greeting", "()Ljava/lang/String;").unwrap();
    let robot = vm.load("Robot").unwrap();
    let (class, _) = carafe::select(&robot, &named, greeting).unwrap();
    assert!(Rc::ptr_eq(&class, &named)); // the default
    let person = vm.load("Person").unwrap();
    let (class, _) = carafe::select(&person, &named, greeting).unwrap();
    assert!(Rc::ptr_eq(&class, &person)); // the override
    // @end
}
