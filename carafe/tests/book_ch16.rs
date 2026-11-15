//! The examples in Chapter 16: `cargo test --test book_ch16`.
use carafe::{Error, Jdk, Vm};
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
/// Runs `program` on carafe and on HotSpot, without HotSpot's detailed
/// `NullPointerException` messages, and returns carafe's result and both
/// JVMs' standard output and standard error.
fn both(program: &str) -> (Result<(), Error>, [String; 2], [String; 2]) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    let result = vm.run_main(program);
    let text = |bytes: &[u8]| String::from_utf8(bytes.to_vec()).unwrap();
    let ours = [text(&vm.stdout), text(&vm.stderr)];
    let option = "-XX:-ShowCodeDetailsInExceptionMessages";
    let (out, err) = jdk.java_both(&[option], &app, program).unwrap();
    (result, ours, [out, err])
}
// @end

#[test]
fn catching() {
    // @listing catching-test
    let (result, ours, theirs) = both("Exceptions");
    assert!(result.is_ok());
    assert_eq!(ours, theirs);
    assert_eq!(ours[0].lines().nth(1), Some("Index 5 out of bounds for length 2"));
    assert!(ours[0].ends_with("t01t2\nnull\n"));
    // @end
}

#[test]
fn the_uncaught_report() {
    // @listing uncaught-test
    let (result, ours, theirs) = both("Uncaught");
    let Err(Error::Thrown { class, .. }) = result else { panic!("{result:?}") };
    assert_eq!(class, "java/lang/ExceptionInInitializerError");
    assert_eq!(ours, theirs);
    assert!(
        ours[1].contains("Caused by: java.lang.IllegalArgumentException: no value")
    );
    // @end
}

#[test]
fn overflowing() {
    // @listing overflow-test
    let (result, ours, theirs) = both("Overflow");
    assert!(result.is_err());
    assert_eq!(ours, theirs);
    assert_eq!(ours[1].lines().count(), 1 + 1024); // the exception, then 1,024 frames
    // @end
}
