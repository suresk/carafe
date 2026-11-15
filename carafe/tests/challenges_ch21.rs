//! Tests for the Chapter 21 challenges: `cargo test --test challenges_ch21`.
use carafe::{Error, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn pipelines() -> Vm {
    let app = compile("Pipelines", "Pipelines").parent().unwrap().to_path_buf();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&Jdk::find().unwrap(), &work, &app).unwrap();
    vm.run_main("Pipelines").unwrap();
    vm
}

// @challenge 1
#[test]
fn where_it_stops() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("boundaries");
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let programs = [
        ("Formatted", "System.out.println(String.format(\"%d\", 42));"),
        (
            "Parallel",
            "System.out.println(java.util.stream.IntStream.range(0, 9).parallel().sum());",
        ),
    ];
    let mut stops = Vec::new();
    for (name, body) in programs {
        let source = dir.join(format!("{name}.java"));
        std::fs::create_dir_all(&dir).unwrap();
        let text = format!(
            "public class {name} {{ public static void main(String[] a) {{ {body} }} }}"
        );
        std::fs::write(&source, text).unwrap();
        jdk.javac(&source, &dir).unwrap();
        let mut vm = Vm::boot(&jdk, &work, &dir).unwrap();
        let Err(error) = vm.run_main(name) else { panic!("{name} ran") };
        let class = match error {
            Error::Thrown { class, .. } => class,
            Error::Exception { class, .. } => class.to_string(),
            other => other.to_string(),
        };
        stops.push(class);
    }
    // String.format needs locale data, found by the JDK's class loaders; a
    // parallel stream needs Unsafe on static fields, and then threads.
    assert_eq!(
        stops,
        ["java/util/ServiceConfigurationError", "java/lang/UnsatisfiedLinkError"]
    );
}

// @challenge 2
use carafe::packages;

#[test]
fn a_census_of_packages() {
    let vm = pipelines();
    let counts = packages(&vm);
    assert_eq!(counts["java/util/stream"], 101);
    let (largest, _) = counts.iter().max_by_key(|&(_, &n)| n).unwrap();
    assert_eq!(largest, "java/util/stream");
}

// @challenge 3
use carafe::lambda_callers;

#[test]
fn where_the_lambdas_come_from() {
    let vm = pipelines();
    let callers = lambda_callers(&vm);
    let callers: Vec<(&str, usize)> =
        callers.iter().map(|(k, &v)| (k.as_str(), v)).collect();
    assert_eq!(
        callers,
        [
            ("Pipelines", 23),
            ("java/util/Comparator", 1),
            ("java/util/function/BinaryOperator", 1),
            ("java/util/function/Function", 1),
            ("java/util/stream/Collectors", 24),
            ("java/util/stream/IntPipeline", 10),
            ("java/util/stream/MatchOps", 1),
            ("java/util/stream/ReferencePipeline", 1),
        ]
    );
}
