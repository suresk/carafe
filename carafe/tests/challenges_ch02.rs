//! Tests for the Chapter 2 challenges: `cargo test --test challenges_ch02`.
use carafe::{AccessFlags, ClassFile, FieldType, Jdk, MethodDescriptor};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn parse(source: &str, class: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile(source, class)).unwrap()).unwrap()
}

/// Every class file the book's Java programs so far compile to.
const CLASSES: [(&str, &str); 10] = [
    ("Hello", "Hello"),
    ("Constants", "Constants"),
    ("Lambdas", "Lambdas"),
    ("Lambdas", "Lambdas$Point"),
    ("Shapes", "Shape"),
    ("Shapes", "Polygon"),
    ("Shapes", "Square"),
    ("Shapes", "Circle"),
    ("Shapes", "Shapes"),
    ("Settings", "Settings"),
];

// @challenge 1
#[test]
fn predict_the_slots() {
    let d = MethodDescriptor::parse("(JLjava/lang/String;[[DZ)[B").unwrap();
    assert_eq!(d.params.len(), 4);
    assert_eq!(d.param_slots(), 5);
    assert_eq!(d.param_slots() + 1, 6); // an instance method's slots, with `this`
    assert_eq!(d.params.len() + 1, 5); // what javap prints as args_size
    assert_eq!(d.ret.unwrap().to_string(), "byte[]");
    assert_eq!(d.params[2], FieldType::parse("[[D").unwrap());
}

// @challenge 2
use carafe::check_flags;

#[test]
fn javac_output_has_legal_flags() {
    for (source, class) in CLASSES {
        check_flags(&parse(source, class)).unwrap();
    }
}

#[test]
fn illegal_flags_are_caught() {
    let mut shape = parse("Shapes", "Shape");
    shape.access_flags.0 |= AccessFlags::FINAL;
    assert!(check_flags(&shape).is_err()); // a final interface

    let mut square = parse("Shapes", "Square");
    square.access_flags.0 |= AccessFlags::ABSTRACT;
    assert!(check_flags(&square).is_err()); // final and abstract

    let mut polygon = parse("Shapes", "Polygon");
    polygon.methods[1].access_flags.0 |= AccessFlags::STATIC;
    assert!(check_flags(&polygon).is_err()); // abstract and static

    let mut circle = parse("Shapes", "Circle");
    circle.fields[0].access_flags.0 |= AccessFlags::PUBLIC;
    assert!(check_flags(&circle).is_err()); // public and private
}

// @challenge 3
use carafe::line_number;

#[test]
fn line_numbers() {
    let shapes = parse("Shapes", "Shapes");
    let main = shapes.method("main", "([Ljava/lang/String;)V").unwrap();
    let code = main.code.as_ref().unwrap();
    let lines: Vec<Option<u16>> = [0, 28, 29, 56, 66, 69, 72, 88]
        .map(|pc| line_number(code, pc).unwrap())
        .into();
    let expected = [64, 64, 65, 67, 66, 66, 69, 70].map(Some);
    assert_eq!(lines, expected);
    let init = shapes.method("<init>", "()V").unwrap();
    assert_eq!(line_number(init.code.as_ref().unwrap(), 4).unwrap(), Some(53));
}

// @challenge 4
use carafe::declaration;

#[test]
fn declarations_match_javap() {
    let jdk = Jdk::find().unwrap();
    for class in ["Shape", "Polygon", "Square", "Circle", "Shapes"] {
        let path = compile("Shapes", class);
        let parsed = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
        let ours: Vec<String> =
            parsed.methods.iter().map(|m| declaration(&parsed, m) + ";").collect();
        let javap = jdk.javap(&path).unwrap();
        let lines: Vec<&str> = javap.lines().map(str::trim).collect();
        let theirs: Vec<&str> = lines
            .windows(2)
            .filter(|w| w[1].starts_with("descriptor: ("))
            .map(|w| w[0])
            .collect();
        assert_eq!(ours, theirs, "{class}");
    }
    let settings = parse("Settings", "Settings");
    let clinit = settings.method("<clinit>", "()V").unwrap();
    assert_eq!(declaration(&settings, clinit), "static {}");
}
