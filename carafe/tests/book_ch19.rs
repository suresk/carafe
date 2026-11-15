//! The examples in Chapter 19: `cargo test --test book_ch19`.
use carafe::{CallSite, ClassPath, Error, Jdk, Label, Vm};
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

/// Runs `program` on carafe and on HotSpot, and returns carafe's VM, what it
/// printed, and what HotSpot printed.
fn both(program: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java(&app, program).unwrap();
    (vm, ours, theirs)
}

#[test]
fn records_and_switches() {
    // @listing kinds-test
    let (_, ours, theirs) = both("Kinds");
    assert_eq!(ours, theirs);
    let lines: Vec<&str> = ours.lines().collect();
    assert_eq!(lines[0], "Circle[radius=1.0]");
    assert_eq!(lines[5], "1072693248 7"); // Double.hashCode(1.0), Integer.hashCode(7)
    assert_eq!(lines[15], "zero one many some ");
    // @end
}

#[test]
fn a_type_switch() {
    // @listing type-switch-test
    let (mut vm, _, _) = both("Kinds");
    let kinds = vm.load("Kinds").unwrap();
    let sites = kinds.call_sites.borrow();
    // size(Integer): case 0, case 1, case Integer i when i > 100, case Integer i
    let size = sites.values().find_map(|site| match &**site {
        CallSite::TypeSwitch { labels } if matches!(labels[0], Label::Int(0)) => {
            Some(labels)
        }
        _ => None,
    });
    let labels = size.unwrap();
    assert!(matches!(
        labels[..],
        [Label::Int(0), Label::Int(1), Label::Type(_), Label::Type(_)]
    ));
    // @end
}

#[test]
fn an_intruder() {
    // @listing intruder-test
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("intruder");
    let jdk = Jdk::find().unwrap();
    let write = |name: &str, text: &str| {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    };
    // Intruder is compiled against a Base that isn't sealed, and run with one that is.
    let sealed = write(
        "sealed/Base.java",
        "sealed class Base permits Allowed {}\n\
                                           final class Allowed extends Base {}\n",
    );
    jdk.javac(&sealed, &dir.join("run")).unwrap();
    let intruder = write(
        "open/Intruder.java",
        "class Base {}\n\
                                                class Intruder extends Base {}\n",
    );
    jdk.javac(&intruder, &dir.join("open")).unwrap();
    std::fs::copy(dir.join("open/Intruder.class"), dir.join("run/Intruder.class"))
        .unwrap();
    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir.join("run")] };
    assert!(vm.load("Allowed").is_ok());
    let Err(Error::Exception { class, message: Some(m) }) = vm.load("Intruder")
    else {
        panic!()
    };
    assert_eq!(class, "java/lang/IncompatibleClassChangeError");
    let hotspot = "Failed listed permitted subclass check: class Intruder is not a \
                   permitted subclass of Base";
    assert_eq!(m, hotspot);
    // @end
}
