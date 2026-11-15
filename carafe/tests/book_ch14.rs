//! The examples in Chapter 14: `cargo test --test book_ch14`.
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

// @listing helpers
/// Runs `program` on carafe and on HotSpot, and returns carafe's VM, what it
/// printed, and what HotSpot printed.
fn both(program: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java_with(&["-Dstdout.encoding=UTF-8"], &app, program).unwrap();
    (vm, ours, theirs)
}
// @end

#[test]
fn printing_an_int() {
    // @listing printing-test
    let (vm, ours, theirs) = both("Printing");
    assert_eq!(ours, "42\n-7\nx\ntrue\n-2147483648\nok\n");
    assert_eq!(ours, theirs);
    let natives: Vec<&str> = vm.natives.iter().map(String::as_str).collect();
    assert!(
        natives.contains(
            &"jdk/internal/misc/Unsafe.arrayBaseOffset0(Ljava/lang/Class;)I"
        )
    );
    assert!(vm.loaded.iter().any(|c| c == "jdk/internal/util/DecimalDigits"));
    // @end
}

#[test]
fn boxing_and_the_library() {
    // @listing library-test
    for program in ["Boxing", "Library", "Counters"] {
        let (_, ours, theirs) = both(program);
        assert_eq!(ours, theirs, "{program}");
    }
    // @end
}

#[test]
fn the_natives_so_far() {
    // @listing natives-test
    let mut called = std::collections::BTreeSet::new();
    for program in ["Printing", "Boxing", "Library", "Counters"] {
        let (vm, _, _) = both(program);
        called.extend(vm.natives);
    }
    let names: Vec<String> =
        called.iter().map(|n| n.split('(').next().unwrap().into()).collect();
    assert_eq!(
        names,
        [
            "java/io/PrintStream.write0",
            "java/lang/Class.desiredAssertionStatus0",
            "java/lang/Class.getPrimitiveClass",
            "java/lang/Class.registerNatives",
            "java/lang/Double.doubleToRawLongBits",
            "java/lang/Float.floatToRawIntBits",
            "java/lang/Object.getClass",
            "java/lang/Runtime.availableProcessors",
            "java/lang/Runtime.maxMemory",
            "java/lang/String.intern",
            "java/lang/System.arraycopy",
            "jdk/internal/misc/CDS.getCDSConfigStatus",
            "jdk/internal/misc/CDS.initializeFromArchive",
            "jdk/internal/misc/Unsafe.arrayBaseOffset0",
            "jdk/internal/misc/Unsafe.arrayIndexScale0",
            "jdk/internal/misc/Unsafe.compareAndSetInt",
            "jdk/internal/misc/Unsafe.compareAndSetLong",
            "jdk/internal/misc/Unsafe.compareAndSetReference",
            "jdk/internal/misc/Unsafe.fullFence",
            "jdk/internal/misc/Unsafe.getIntVolatile",
            "jdk/internal/misc/Unsafe.getLongVolatile",
            "jdk/internal/misc/Unsafe.getReferenceVolatile",
            "jdk/internal/misc/Unsafe.objectFieldOffset1",
            "jdk/internal/misc/Unsafe.putByte",
            "jdk/internal/misc/Unsafe.putReferenceVolatile",
            "jdk/internal/misc/Unsafe.registerNatives",
            "jdk/internal/misc/VM.initialize",
        ]
    );
    assert_eq!(names.len(), 27);
    // @end
}
