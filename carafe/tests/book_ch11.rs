//! The examples in Chapter 11: `cargo test --test book_ch11`.
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
const CLASSES: &str = "Dispatch Dispatch$Inner Animal Dog Puppy Shape2 Circle2";

/// A directory of its own for `test`, holding the classes of `Dispatch.java`.
fn class_dir(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch11").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    for class in CLASSES.split_whitespace() {
        let to = dir.join(format!("{class}.class"));
        std::fs::copy(compile("Dispatch", class), to).unwrap();
    }
    dir
}

/// Runs `Dispatch` on carafe and on HotSpot: carafe's output and error, and
/// what HotSpot printed, its stack trace left out.
fn both(dir: &Path) -> (String, Option<String>, String) {
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, dir).unwrap();
    let error = vm.run_main("Dispatch").err().map(|e| e.to_string());
    let ours = String::from_utf8(vm.stdout).unwrap();
    let theirs = match jdk.java(dir, "Dispatch") {
        Ok(out) => out,
        Err(carafe::Error::Jdk(failure)) => failure,
        Err(e) => panic!("{e}"),
    };
    (ours, error, theirs)
}
// @end

#[test]
fn three_levels() {
    // @listing dispatch-test
    let (ours, error, theirs) = both(&class_dir("dispatch"));
    assert_eq!(error, None);
    let lines: Vec<&str> = ours.lines().collect();
    assert_eq!(
        lines[6..],
        [
            "...",
            "Woof",
            "Yip", // speak, on each of the three
            "Puppy.origin",
            "Dog.origin",
            "Animal.origin", // super.origin()
            "Animal.secret", // a private method isn't overridden
            "Yip",           // describe() calls speak() on the Puppy
            "Dispatch.whisper",
            "circle",
        ]
    );
    assert_eq!(ours, theirs);
    // @end
}

#[test]
fn selection_from_rust() {
    // @listing select-test
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &class_dir("select")).unwrap();
    let animal = vm.load("Animal").unwrap();
    let speak = animal.method("speak", "()V").unwrap();
    let secret = animal.method("secret", "()V").unwrap();
    for (receiver, speaks) in
        [("Animal", "Animal"), ("Dog", "Dog"), ("Puppy", "Puppy")]
    {
        let receiver = vm.load(receiver).unwrap();
        let (class, _) = carafe::select(&receiver, &animal, speak).unwrap();
        assert_eq!(class.file.this_class, speaks);
        let (class, _) = carafe::select(&receiver, &animal, secret).unwrap();
        assert!(Rc::ptr_eq(&class, &animal)); // private: always Animal's
    }
    // @end
}

#[test]
fn a_missing_implementation() {
    // @listing abstract-test
    let dir = class_dir("abstract");
    let path = dir.join("Circle2.class");
    let bytes = std::fs::read(&path).unwrap();
    let at = bytes.windows(4).position(|w| w == b"kind").unwrap();
    let mut bytes = bytes;
    bytes[at + 3] = b'x'; // Circle2 now declares kinx, not kind
    std::fs::write(&path, bytes).unwrap();
    let (_, error, theirs) = both(&dir);
    let error = error.unwrap();
    assert_eq!(
        error,
        "java.lang.AbstractMethodError: Receiver class Circle2 does not define or \
         inherit an implementation of the resolved method 'abstract java.lang.String \
         kind()' of abstract class Shape2."
    );
    assert!(theirs.contains(&error));
    // @end
}
