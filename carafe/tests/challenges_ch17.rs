//! Tests for the Chapter 17 challenges: `cargo test --test challenges_ch17`.
use carafe::{ClassFile, Constant, Jdk, bootstrap_methods};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

// @challenge 1
#[test]
fn predict_the_recipe() {
    let file = ClassFile::parse(&std::fs::read(compile("Concat", "Concat")).unwrap())
        .unwrap();
    let pool = &file.constant_pool;
    let recipes: Vec<String> = bootstrap_methods(&file)
        .unwrap()
        .iter()
        .map(|b| {
            let Constant::String { utf8 } = *pool.get(b.args[0]).unwrap() else {
                panic!()
            };
            pool.utf8(utf8).unwrap().to_string()
        })
        .collect();
    // "" + min + Integer.MIN_VALUE + Character.MAX_VALUE: the constants are folded
    // into the recipe's text, and the variable is an argument.
    assert!(recipes.contains(&"\u{1}-2147483648\u{ffff}".to_string()));
    // "ok " + 4.0 is a constant expression, so it has no recipe at all.
    assert!(!recipes.iter().any(|r| r.contains("4.0")));
}

// @challenge 2
use carafe::bootstrap_listing;

#[test]
fn bootstrap_methods_as_javap_prints_them() {
    let jdk = Jdk::find().unwrap();
    for (source, class) in
        [("Concat", "Concat"), ("Lambdas", "Lambdas"), ("Lambdas", "Lambdas$Point")]
    {
        let path = compile(source, class);
        let file = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
        let javap = jdk.javap(&path).unwrap();
        let start = javap.find("BootstrapMethods:\n").unwrap();
        let section: String = javap[start..]
            .lines()
            .enumerate()
            .take_while(|&(n, line)| n == 0 || line.starts_with(' '))
            .map(|(_, line)| format!("{line}\n"))
            .collect();
        assert_eq!(bootstrap_listing(&file).unwrap(), section, "{class}");
    }
}

// @challenge 3
use carafe::bootstrap_census;

#[test]
fn bootstraps_in_java_base() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let java_base = Jdk::find().unwrap().java_base(&work).unwrap();
    let counts = bootstrap_census(&java_base).unwrap();
    let counts: Vec<(&str, usize)> =
        counts.iter().map(|(k, &v)| (k.as_str(), v)).collect();
    // java.base's platform-specific classes differ between Linux and macOS.
    let (lambdas, records) =
        if cfg!(target_os = "linux") { (1563, 515) } else { (1546, 518) };
    assert_eq!(
        counts,
        [
            ("java/lang/invoke/LambdaMetafactory.altMetafactory", 15),
            ("java/lang/invoke/LambdaMetafactory.metafactory", lambdas),
            ("java/lang/runtime/ObjectMethods.bootstrap", records),
            ("java/lang/runtime/SwitchBootstraps.typeSwitch", 64),
        ]
    );
}
