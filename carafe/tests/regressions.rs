//! Behavior that no chapter's own program reaches, checked against HotSpot
//! wherever HotSpot defines it: `cargo test --test regressions`. Not printed
//! in the book.
use carafe::{Class, ClassFile, Code, Constant, Frame, Jdk, Value, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The classes `Regressions.java` declares and uses, `Missing` left out.
const CLASSES: &str = "Regressions Defaulted Middle Bottom FinalBase FromFinal \
                       NotAnInterface Implementor Secretive Caller Holder Outsider \
                       MissingUser Face FaceImpl SuperS SubS p/Hidden p/Same \
                       q/Peek q/Base q/J p/Sub p/Impl";

/// A directory holding the classes of `Regressions.java`, `Missing` deleted
/// and the rest patched as the source's comments say.
fn class_dir() -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join("Regressions.java"), &out).unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("regressions");
    for class in CLASSES.split_whitespace() {
        let to = dir.join(format!("{class}.class"));
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(out.join(format!("{class}.class")), to).unwrap();
    }
    let class = |name: &str| dir.join(format!("{name}.class"));
    set_class_flags(&class("FinalBase"), 0x0030); // ACC_SUPER | ACC_FINAL
    set_class_flags(&class("NotAnInterface"), 0x0420); // ACC_SUPER | ACC_ABSTRACT
    set_class_flags(&class("p/Hidden"), 0x0020); // not ACC_PUBLIC
    set_class_flags(&class("q/Base"), 0x0020);
    set_class_flags(&class("q/J"), 0x0600); // ACC_INTERFACE | ACC_ABSTRACT
    set_member_flags(&class("Secretive"), "m", 0x000a); // ACC_PRIVATE | ACC_STATIC
    set_member_flags(&class("Holder"), "value", 0x0010); // ACC_FINAL
    set_member_flags(&class("Holder"), "count", 0x0018); // ACC_STATIC | ACC_FINAL
    set_member_flags(&class("Face"), "other", 0x0009); // ACC_PUBLIC | ACC_STATIC
    set_member_flags(&class("FaceImpl"), "run", 0x0000); // package-private
    set_member_flags(&class("SuperS"), "m", 0x0008); // ACC_STATIC
    dir
}

/// The index of the `Utf8` entry `text` in the pool of the class file `bytes`.
fn utf8_index(bytes: &[u8], text: &str) -> u16 {
    let pool = ClassFile::parse(bytes).unwrap().constant_pool;
    let utf8 = Constant::Utf8(text.to_string());
    pool.iter().find(|(_, c)| **c == utf8).unwrap().0
}

/// Overwrites the access flags of the class in the class file at `path`,
/// which come just before `this_class` and `super_class`.
fn set_class_flags(path: &Path, flags: u16) {
    let mut bytes = std::fs::read(path).unwrap();
    let file = ClassFile::parse(&bytes).unwrap();
    let class_index = |name: &str| {
        let pool = &file.constant_pool;
        let found =
            pool.iter().find(|&(i, _)| pool.class_name(i).is_ok_and(|n| n == name));
        found.unwrap().0
    };
    let this = class_index(&file.this_class);
    let sup = class_index(file.super_class.as_deref().unwrap());
    let mut header = file.access_flags.0.to_be_bytes().to_vec();
    header.extend(this.to_be_bytes());
    header.extend(sup.to_be_bytes());
    let at = bytes.windows(6).position(|w| w == header).unwrap();
    bytes[at..at + 2].copy_from_slice(&flags.to_be_bytes());
    std::fs::write(path, bytes).unwrap();
}

/// Overwrites the access flags of the field or method `name` in the class
/// file at `path`, as Chapter 8's `set_flags` does: the last use of the
/// name's index is the member's, and its flags are the two bytes before it.
fn set_member_flags(path: &Path, name: &str, flags: u16) {
    let mut bytes = std::fs::read(path).unwrap();
    let index = utf8_index(&bytes, name).to_be_bytes();
    let at = bytes.windows(2).rposition(|w| w == index).unwrap() - 2;
    bytes[at..at + 2].copy_from_slice(&flags.to_be_bytes());
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn linkage_rules_as_hotspot_applies_them() {
    let dir = class_dir();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
    vm.run_main("Regressions").unwrap();
    let ours = String::from_utf8(vm.stdout).unwrap();
    let iae = "java.lang.IllegalAccessError";
    let icce = "java.lang.IncompatibleClassChangeError";
    let expected = [
        "7".to_string(),
        "ok".to_string(),
        format!("{icce}: class FromFinal cannot inherit from final class FinalBase"),
        format!(
            "{icce}: class Implementor can not implement NotAnInterface, because it \
             is not an interface"
        ),
        format!(
            "{iae}: class Caller tried to access private method 'int Secretive.m()'"
        ),
        format!("{iae}: class p.Sub cannot access its superclass q.Base"),
        format!("{iae}: class p.Impl cannot access its superinterface q.J"),
        format!(
            "{iae}: Update to non-static final field Holder.value attempted from a \
             different method (set) than the initializer method <init> "
        ),
        format!(
            "{iae}: Update to non-static final field Holder.value attempted from a \
             different class (Outsider) than the field's declaring class"
        ),
        format!(
            "{iae}: Update to static final field Holder.count attempted from a \
             different method (bump) than the initializer method <clinit> "
        ),
        format!(
            "{iae}: Update to static final field Holder.count attempted from a \
             different class (Outsider) than the field's declaring class"
        ),
        "0".to_string(),
        "ok".to_string(),
        "null".to_string(),
        "ok".to_string(),
        format!("{iae}: failed to access class p.Hidden from class q.Peek"),
        "cast true".to_string(),
        "ok".to_string(),
        "false".to_string(),
        "ok".to_string(),
        format!("{iae}: 'void FaceImpl.run()'"),
        format!("{icce}: Expected instance not static method 'void Face.other()'"),
        format!("{icce}: Expecting non-static method 'void SuperS.m()'"),
    ];
    let ours: Vec<&str> = ours.lines().collect();
    assert_eq!(ours, expected);

    // HotSpot prints the same, with a description of the classes' module and
    // loader after some of the messages.
    let theirs = jdk.java(&dir, "Regressions").unwrap();
    let theirs: Vec<&str> = theirs.lines().collect();
    assert_eq!(ours.len(), theirs.len());
    for (ours, theirs) in ours.iter().zip(&theirs) {
        let rest = theirs.strip_prefix(ours);
        assert!(
            rest.is_some_and(|r| r.is_empty() || r.starts_with(" (")),
            "{theirs}"
        );
    }
}

#[test]
fn static_constraints_on_operands() {
    let decode = |bytecode: &[u8]| {
        let code = Code {
            max_stack: 4,
            max_locals: 4,
            bytecode: bytecode.to_vec(),
            handlers: Vec::new(),
            attributes: Vec::new(),
        };
        code.decode().unwrap_err().to_string()
    };
    let error = |m: &str| format!("ClassFormatError: {m}");
    assert_eq!(
        decode(&[0xbc, 3, 0xb1]),
        error("the opcode 0xbc at 0 has array type 3")
    );
    assert_eq!(
        decode(&[0xc5, 0, 1, 0, 0xb1]),
        error("the opcode 0xc5 at 0 has 0 dimensions")
    );
    assert_eq!(
        decode(&[0xb9, 0, 1, 0, 0, 0xb1]),
        error("the opcode 0xb9 at 0 needs a count and then a zero byte")
    );
    assert_eq!(
        decode(&[0xba, 0, 1, 0, 1, 0xb1]),
        error("the opcode 0xba at 0 needs two zero bytes after its index")
    );
    assert_eq!(
        decode(&[0xc4, 0xa9, 0, 1, 0xb1]),
        error("the opcode 0xa9 at 1 is not allowed after Java 6")
    );
}

#[test]
fn halves_of_a_long() {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    Jdk::find().unwrap().javac(&java.join("Arith.java"), &out).unwrap();
    let bytes = std::fs::read(out.join("Arith.class")).unwrap();
    let arith = Rc::new(Class::new(ClassFile::parse(&bytes).unwrap()));
    let sum = arith.method("sum", "(I)I").unwrap();
    let mut frame = Frame::new(arith.clone(), sum, &[Value::Int(10)]).unwrap();

    // A `long` on top of the stack without its `Top` is half of a value.
    frame.stack = vec![Value::Top, Value::Long(7)];
    let error = frame.pop().unwrap_err().to_string();
    assert_eq!(error, "VerifyError: sum pops half of a value");

    // An `int` stored over the second half of a `long` kills the first half.
    frame.store(0, Value::Long(5)).unwrap();
    frame.store(1, Value::Int(3)).unwrap();
    assert!(frame.load(0).is_err());
    assert_eq!(frame.load(1).unwrap(), Value::Int(3));
}

/// Runs `program`, compiled from `java/<program>.java`, on carafe and on
/// HotSpot with `options`, and returns what each printed to standard output
/// and to standard error.
fn both(program: &str, options: &[&str]) -> ((String, String), (String, String)) {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{program}.java")), &out).unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &out).unwrap();
    let result = vm.run_main(program);
    assert!(
        matches!(result, Ok(()) | Err(carafe::Error::Thrown { .. })),
        "{result:?}"
    );
    let text = |bytes: &[u8]| String::from_utf8(bytes.to_vec()).unwrap();
    let ours = (text(&vm.stdout), text(&vm.stderr));
    let mut options = options.to_vec();
    options.push("-XX:-ShowCodeDetailsInExceptionMessages");
    let theirs = jdk.java_both(&options, &out, program).unwrap();
    (ours, theirs)
}

#[test]
fn monitors_are_owned() {
    let ((out, err), theirs) = both("RvMonitors", &[]);
    let imse = "java.lang.IllegalMonitorStateException";
    let expected = [
        format!("notify: {imse}: current thread is not owner"),
        format!("notifyAll: {imse}: current thread is not owner"),
        format!("wait: {imse}: current thread is not owner"),
        "held notify ok".to_string(),
        "nested ok".to_string(),
        "static sync notify ok".to_string(),
        "sync wait(1) ok".to_string(),
        "held after astore ok".to_string(),
        "thrown from a synchronized method".to_string(),
        format!("released: {imse}: current thread is not owner"),
        "wait(-1): java.lang.IllegalArgumentException: timeout value is negative"
            .to_string(),
    ];
    assert_eq!(out.lines().collect::<Vec<_>>(), expected);
    assert_eq!((out, err), theirs);
}

#[test]
fn array_and_synthetic_modifiers() {
    let ((out, _), (theirs, _)) = both("RvMods", &[]);
    assert_eq!(out, "414\n412\n414\nc\n411\n1010\n");
    assert_eq!(out, theirs);
}

#[test]
fn unsafe_stays_inside_the_object() {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("unsafe");
    let home =
        std::env::var_os("CARAFE_JDK").or_else(|| std::env::var_os("JAVA_HOME"));
    let javac = Path::new(&home.unwrap()).join("bin").join("javac");
    let status = std::process::Command::new(javac)
        .args(["--add-exports", "java.base/jdk.internal.misc=ALL-UNNAMED", "-d"])
        .arg(&out)
        .arg(java.join("RvUnsafe.java"))
        .status()
        .unwrap();
    assert!(status.success());
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &out).unwrap();
    vm.run_main("RvUnsafe").unwrap();
    let outside = |at: i64| format!("Unsafe access at offset {at} outside 24 bytes");
    let expected =
        format!("7\n{}\n{}\n{}\n5\n", outside(24), outside(1 << 30), outside(-8));
    assert_eq!(String::from_utf8(vm.stdout).unwrap(), expected);
}

#[test]
fn recipes_that_dont_fit() {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join("RvRecipe.java"), &out).unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("recipes");
    std::fs::create_dir_all(&dir).unwrap();
    let mut bytes = std::fs::read(out.join("RvRecipe.class")).unwrap();
    for (from, to) in
        [(b"<\x01|\x01>", b"<\x01|?>"), (b"[\x01|\x01]", b"[\x01\x02\x01]")]
    {
        let at = bytes.windows(5).position(|w| w == from).unwrap();
        bytes[at..at + 5].copy_from_slice(to);
    }
    std::fs::write(dir.join("RvRecipe.class"), bytes).unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
    vm.run_main("RvRecipe").unwrap();
    let ours = String::from_utf8(vm.stdout).unwrap();
    let error =
        "java.lang.BootstrapMethodError: bootstrap method initialization exception";
    let cause = "java.lang.invoke.StringConcatException: Mismatched number of concat";
    let expected = format!(
        "{error}\n{cause} arguments: recipe wants 1 arguments, but signature \
         provides 2\n{error}\n{cause} constants: recipe wants 0 constants, but \
         only 0 are passed\n"
    );
    assert_eq!(ours, expected);
    assert_eq!(ours, jdk.java(&dir, "RvRecipe").unwrap());
}

/// Copies the class file of `class` into `dir`, with the kind of the
/// `MethodHandle` constant for the method `name` set to `kind`.
fn patch_handle(class: &str, name: &str, kind: u8, dir: &Path) {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    Jdk::find().unwrap().javac(&java.join(format!("{class}.java")), &out).unwrap();
    let mut bytes = std::fs::read(out.join(format!("{class}.class"))).unwrap();
    let file = ClassFile::parse(&bytes).unwrap();
    let pool = &file.constant_pool;
    let (old, reference) = pool
        .iter()
        .find_map(|(_, c)| match *c {
            Constant::MethodHandle { kind, reference }
                if pool.member(reference).unwrap().name == name =>
            {
                Some((kind, reference))
            }
            _ => None,
        })
        .unwrap();
    let [hi, lo] = reference.to_be_bytes();
    let at = bytes.windows(4).position(|w| w == [15, old, hi, lo]).unwrap();
    bytes[at + 1] = kind;
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(format!("{class}.class")), bytes).unwrap();
}

#[test]
fn lambda_handles_and_unboxing() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("handles");
    patch_handle("RvLambda", "plus", 7, &dir); // REF_invokeSpecial
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
    vm.run_main("RvLambda").unwrap();
    let ours = String::from_utf8(vm.stdout).unwrap();
    assert_eq!(ours, "5\n7\n42\n");
    assert_eq!(ours, jdk.java(&dir, "RvLambda").unwrap());

    patch_handle("RvSpecial", "length", 7, &dir);
    let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
    let error = vm.run_main("RvSpecial").unwrap_err().to_string();
    assert!(error.ends_with("a REF_invokeSpecial handle to java/lang/String.length"));
}

#[test]
fn dynamic_constants_as_switch_labels() {
    let ((out, err), theirs) = both("RvSwitch", &[]);
    assert_eq!(out, "1\n3\n2\n3\n3\n1\n3\n2\n");
    assert_eq!((out, err), theirs);
}

#[test]
fn method_invoke_checks_and_widens() {
    let ((out, err), theirs) = both("RvReflect", &[]);
    let iae = "java.lang.IllegalArgumentException";
    let mismatch = format!("{iae}: argument type mismatch");
    let expected = format!(
        "{iae}: object of type java.lang.String is not an instance of RvReflect\n\
         3\n42\n194\n2.5\n{mismatch}\n{mismatch}\n{mismatch}\n\
         {iae}: java.lang.NullPointerException\n"
    );
    assert_eq!(out, expected);
    assert_eq!((out, err), theirs);
}

#[test]
fn field_access_needs_java_lang_invoke() {
    let ((out, _), (theirs, _)) = both("RvField", &[]);
    let error = "java.lang.InternalError: RvField::x cannot be accessed \
                 reflectively before java.lang.invoke is initialized";
    assert_eq!(out, format!("{error}\n{error}\n"));
    assert_eq!(theirs, "7\n");
}
