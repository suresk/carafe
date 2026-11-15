//! The examples in Chapter 20: `cargo test --test book_ch20`.
use carafe::{Jdk, Vm};
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
fn reflection() {
    // @listing reflect-test
    let (_, ours, theirs) = both("Reflect");
    assert_eq!(ours, theirs);
    let lines: Vec<&str> = ours.lines().collect();
    assert_eq!(lines[..2], ["2", "No enum constant Reflect.Planet.PLUTO"]);
    assert_eq!(lines[4], "public int Reflect$Counter.add(int)");
    assert_eq!(lines[10], "java.lang.IllegalStateException: failed on purpose");
    // @end
}

#[test]
fn the_natives_of_reflection() {
    // @listing reflect-natives-test
    let (vm, _, _) = both("Reflect");
    let reflection = |n: &&String| {
        n.contains("reflect") || n.starts_with("java/lang/Class.") && n.contains("0(")
    };
    let names: Vec<&str> = vm
        .natives
        .iter()
        .filter(reflection)
        .map(|n| n.split('(').next().unwrap().rsplit('/').next().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "Class.desiredAssertionStatus0",
            "Class.forName0",
            "Class.getConstantPool",
            "Class.getDeclaredConstructors0",
            "Class.getDeclaredMethods0",
            "Class.getDeclaringClass0",
            "Class.getEnclosingMethod0",
            "Class.getSimpleBinaryName0",
            "DirectConstructorHandleAccessor$NativeAccessor.newInstance0",
            "DirectMethodHandleAccessor$NativeAccessor.invoke0",
            "Reflection.getCallerClass",
            "Reflection.getClassAccessFlags",
        ]
    );
    // @end
}
