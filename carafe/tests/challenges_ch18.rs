//! Tests for the Chapter 18 challenges: `cargo test --test challenges_ch18`.
use carafe::{CallSite, Class, Jdk, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

/// Runs `Functional` and returns the VM and the `Functional` class.
fn functional() -> (Vm, Rc<Class>) {
    let app = compile("Functional", "Functional").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main("Functional").unwrap();
    let class = vm.load("Functional").unwrap();
    (vm, class)
}

/// The lambda class called `name`, linked by a call site in `class`.
fn lambda_class(class: &Class, name: &str) -> Rc<Class> {
    let sites = class.call_sites.borrow();
    let found = sites.values().find_map(|site| match &**site {
        CallSite::Lambda { class, .. } if class.file.this_class == name => {
            Some(class.clone())
        }
        _ => None,
    });
    found.unwrap()
}

// @challenge 1
#[test]
fn predict_the_adaptation() {
    let (_, class) = functional();
    // ToLongFunction<String> longLength = String::length
    let lambda = lambda_class(&class, "Functional$$Lambda$4");
    let code = lambda.instructions(0).unwrap();
    let mnemonics: Vec<String> = code.iter().map(|i| i.mnemonic()).collect();
    assert_eq!(mnemonics, ["aload", "checkcast", "invokevirtual", "i2l", "lreturn"]);
}

// @challenge 2
use carafe::captures;

#[test]
fn what_lambdas_capture() {
    let (_, class) = functional();
    let found = captures(&class);
    let capturing: Vec<(&str, &[String])> = found
        .iter()
        .filter(|(_, c)| !c.is_empty())
        .map(|(name, c)| (name.as_str(), &c[..]))
        .collect();
    assert_eq!(
        capturing,
        [
            ("Functional$$Lambda$14", &["LFunctional;".to_string()][..]),
            ("Functional$$Lambda$1", &["I".to_string()][..]),
            ("Functional$$Lambda$6", &["Ljava/io/PrintStream;".to_string()][..]),
        ]
    );
}

// @challenge 3
use carafe::write_class;

#[test]
fn javap_a_lambda_class() {
    let (_, class) = functional();
    let lambda = lambda_class(&class, "Functional$$Lambda$2");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lambda-classes");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Functional$$Lambda$2.class");
    std::fs::write(&path, write_class(&lambda.file).unwrap()).unwrap();
    let javap = Jdk::find().unwrap().javap(&path).unwrap();
    assert!(javap.contains("final class Functional$$Lambda$2 implements java.util.function.BinaryOperator"));
    assert!(javap.contains("// Method java/lang/Integer.sum:(II)I"));
    assert!(javap.contains("NestHost: class Functional"));
}
