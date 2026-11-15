//! Tests for the Chapter 3 challenges: `cargo test --test challenges_ch03`.
use carafe::{ClassFile, Code, Jdk, Operand};
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

fn code(class: &ClassFile, name: &str) -> Code {
    let method = class.methods.iter().find(|m| m.name == name).unwrap();
    method.code.clone().unwrap()
}

/// A `Code` attribute with this bytecode and nothing else.
fn bare(bytecode: &[u8]) -> Code {
    let (handlers, attributes) = (Vec::new(), Vec::new());
    Code {
        max_stack: 0,
        max_locals: 0,
        bytecode: bytecode.to_vec(),
        handlers,
        attributes,
    }
}

/// Every class file the book's Java programs so far compile to.
const CLASSES: [(&str, &str); 12] = [
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
    ("Switches", "Switches"),
    ("Catches", "Catches"),
];

// @challenge 1
#[test]
fn predict_the_lengths() {
    let class = parse("Switches", "Switches");
    let days = code(&class, "days").decode().unwrap();
    assert_eq!(days[2].pc - days[1].pc, 1 + 2 + 12 + 4 * 10);
    let size = code(&class, "size").decode().unwrap();
    assert_eq!(size[2].pc - size[1].pc, 1 + 2 + 8 + 8 * 3);
    let Operand::TableSwitch { targets, .. } = &days[1].operand else { panic!() };
    assert_eq!(targets.iter().filter(|&&t| t == 59).count(), 4); // 30 days
}

// @challenge 2
use carafe::encode_bytecode;

#[test]
fn bytecode_writes_back_byte_for_byte() {
    let mut methods = 0;
    for (source, name) in CLASSES {
        let class = parse(source, name);
        for code in class.methods.iter().filter_map(|m| m.code.as_ref()) {
            assert_eq!(encode_bytecode(&code.decode().unwrap()), code.bytecode);
            methods += 1;
        }
    }
    assert_eq!(methods, 38);
}

// @challenge 3
use carafe::exception_table;

#[test]
fn exception_tables_match_javap() {
    let path = compile("Catches", "Catches");
    let class = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
    let words = |line: &str| line.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut ours = Vec::new();
    for code in class.methods.iter().filter_map(|m| m.code.as_ref()) {
        ours.extend(exception_table(&class, code).unwrap().iter().map(|l| words(l)));
    }
    let javap = Jdk::find().unwrap().javap_code(&path).unwrap();
    let theirs: Vec<String> = javap
        .split("Exception table:\n")
        .skip(1)
        .flat_map(|t| {
            let lines = t.split("\n\n").next().unwrap().lines();
            ["Exception table:".to_string()].into_iter().chain(lines.map(words))
        })
        .filter(|l| l != "}")
        .collect();
    assert_eq!(ours, theirs);
    assert_eq!(ours.len(), 9);
}

// @challenge 4
use carafe::unreachable;

#[test]
fn unreachable_code() {
    for (source, name) in CLASSES {
        let class = parse(source, name);
        for code in class.methods.iter().filter_map(|m| m.code.as_ref()) {
            assert_eq!(unreachable(code).unwrap(), [], "{name}");
        }
    }
    // iconst_0, ireturn, then an iconst_1 that nothing jumps to
    assert_eq!(unreachable(&bare(&[0x03, 0xac, 0x04, 0xac])).unwrap(), [2, 3]);
    // goto 4 jumps over the iconst_1 at 3 to the ireturn at 4
    let skip = [0xa7, 0x00, 0x04, 0x04, 0xac, 0x03, 0xac];
    assert_eq!(unreachable(&bare(&skip)).unwrap(), [3, 5, 6]);
}
