//! Tests for the Chapter 16 challenges: `cargo test --test challenges_ch16`.
use carafe::{ClassFile, Error, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn boot(program: &str) -> Vm {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap()
}

// @challenge 1
#[test]
fn predict_the_finally_copies() {
    let bytes = std::fs::read(compile("Exceptions", "Exceptions")).unwrap();
    let class = ClassFile::parse(&bytes).unwrap();
    let method = class.methods.iter().find(|m| m.name == "finallyOrder").unwrap();
    let code = method.code.as_ref().unwrap();
    // Two ranges that run the try block, both sent to one handler for any exception.
    assert_eq!(code.handlers.len(), 2);
    assert!(code.handlers.iter().all(|h| h.catch_type == 0 && h.handler_pc == 45));
    // The finally block, `order.append(i)`, is in the code three times.
    let append = [0x2a, 0x1b, 0xb6]; // aload_0, iload_1, invokevirtual
    assert_eq!(code.bytecode.windows(3).filter(|w| *w == append).count(), 3);
}

// @challenge 2
use carafe::{MAX_FRAMES, static_int};

#[test]
fn how_deep() {
    let mut vm = boot("Exceptions");
    vm.run_main("Exceptions").unwrap();
    // main is one frame, and every other frame was a call to recurse.
    assert_eq!(
        static_int(&mut vm, "Exceptions", "depth").unwrap(),
        MAX_FRAMES as i32 - 1
    );
}

// @challenge 3
use carafe::full_report;

#[test]
fn suppressed_in_the_report() {
    let mut vm = boot("Suppressed");
    let Err(Error::Thrown { exception, .. }) = vm.run_main("Suppressed") else {
        panic!()
    };
    let ours = full_report(&mut vm, exception).unwrap();
    let app = compile("Suppressed", "Suppressed").parent().unwrap().to_path_buf();
    let (_, theirs) =
        Jdk::find().unwrap().java_both(&[], &app, "Suppressed").unwrap();
    assert_eq!(ours, theirs);
    assert!(ours.contains("\n\tSuppressed: java.lang.IllegalStateException: a\n"));
}
