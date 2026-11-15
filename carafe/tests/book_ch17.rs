//! The examples in Chapter 17: `cargo test --test book_ch17`.
use carafe::{CallSite, Jdk, Piece, Vm, bootstrap_methods, opcode::INVOKEDYNAMIC};
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
fn concatenation() {
    // @listing concat-test
    let (_, ours, theirs) = both("Concat");
    assert_eq!(ours, theirs);
    assert!(
        ours.starts_with("ok 4.0\nok 4.0\ncount=3, big=1099511627776, half=0.5\n")
    );
    assert!(ours.contains("\n-0.0|NaN|0.1|3.1415927\n3.4028235E38|1.0E-10\n"));
    // @end
}

#[test]
fn linked_once() {
    // @listing linked-once-test
    let (mut vm, _, _) = both("Concat");
    let class = vm.load("Concat").unwrap();
    let sites = (0..class.file.methods.len())
        .filter_map(|m| class.instructions(m).ok())
        .map(|code| code.iter().filter(|i| i.opcode == INVOKEDYNAMIC).count())
        .sum::<usize>();
    assert_eq!(sites, 11);
    // Each call site is linked once, however often it ran.
    assert_eq!(class.call_sites.borrow().len(), sites);
    assert_eq!(bootstrap_methods(&class.file).unwrap().len(), 10);
    // @end
}

#[test]
fn a_recipe() {
    // @listing recipe-test
    let (mut vm, _, _) = both("Concat");
    let class = vm.load("Concat").unwrap();
    let sites = class.call_sites.borrow();
    let text = |s: &str| Piece::Text(s.encode_utf16().collect());
    let wanted = [text("a\u{1}b"), Piece::Arg(0), text("\u{2}"), Piece::Arg(1)];
    let found = sites.values().any(|site| {
        matches!(&**site, CallSite::Concat { pieces, .. } if pieces[..] == wanted[..])
    });
    assert!(found); // "a\u0001b" + count + "\u0002" + c
    // @end
}
