//! The examples in Chapter 3: `cargo test --test book_ch03`.
use carafe::opcode::{self, MNEMONICS};
use carafe::{ClassFile, Code, Instruction, Jdk, Operand};
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
/// Parses the class file for `class`, compiled from `java/<source>.java`.
fn parse(source: &str, class: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile(source, class)).unwrap()).unwrap()
}

/// The decoded instructions of the method `name` (the first, if overloaded).
fn instructions(class: &ClassFile, name: &str) -> Vec<Instruction> {
    let method = class.methods.iter().find(|m| m.name == name).unwrap();
    method.code.as_ref().unwrap().decode().unwrap()
}

/// A `Code` attribute with this bytecode and nothing else.
fn code(bytecode: &[u8]) -> Code {
    let (max_stack, max_locals) = (0, 0);
    let (handlers, attributes) = (Vec::new(), Vec::new());
    Code { max_stack, max_locals, bytecode: bytecode.to_vec(), handlers, attributes }
}
// @end

#[test]
fn the_opcodes() {
    // @listing opcodes-test
    assert_eq!(MNEMONICS.iter().filter(|m| !m.is_empty()).count(), 202);
    assert_eq!((opcode::IADD, MNEMONICS[0x60]), (0x60, "IADD"));
    assert_eq!(MNEMONICS[usize::from(opcode::JSR_W)], "JSR_W");
    assert_eq!(MNEMONICS[0xca], ""); // breakpoint, reserved for debuggers
    // @end
}

#[test]
fn hello_decoded() {
    // @listing hello-test
    let main = instructions(&parse("Hello", "Hello"), "main");
    let summary: Vec<(u16, String, Operand)> =
        main.iter().map(|i| (i.pc, i.mnemonic(), i.operand.clone())).collect();
    assert_eq!(
        summary,
        [
            (0, "getstatic".to_string(), Operand::Constant(7)),
            (3, "ldc".to_string(), Operand::Constant(13)),
            (5, "invokevirtual".to_string(), Operand::Constant(15)),
            (8, "return".to_string(), Operand::None),
        ]
    );
    // @end
}

#[test]
fn switches() {
    // @listing switch-test
    let class = parse("Switches", "Switches");
    let days = instructions(&class, "days");
    assert_eq!((days[1].pc, days[2].pc), (1, 56));
    let Operand::TableSwitch { default, low, targets } = &days[1].operand else {
        panic!("not a tableswitch")
    };
    assert_eq!((*default, *low, targets.len()), (62, 2, 10));
    assert_eq!(targets[..3], [56, 62, 59]); // February, March, April

    let size = instructions(&class, "size");
    let Operand::LookupSwitch { default, pairs } = &size[1].operand else {
        panic!("not a lookupswitch")
    };
    assert_eq!((*default, &pairs[..]), (45, &[(-5, 36), (10, 39), (1000, 42)][..]));
    // @end
}

#[test]
fn wide_increments() {
    // @listing wide-test
    let total = instructions(&parse("Switches", "Switches"), "total");
    let iinc = total.iter().find(|i| i.opcode == opcode::IINC).unwrap();
    assert_eq!((iinc.pc, iinc.mnemonic()), (20, "iinc_w".to_string()));
    assert_eq!(iinc.operand, Operand::Iinc { local: 3, delta: 300 });
    let next = total.iter().find(|i| i.pc > iinc.pc).unwrap();
    assert_eq!(next.pc, 26); // wide, iinc, a two-byte slot and a two-byte delta
    // @end
}

#[test]
fn bad_bytecode() {
    // @listing bad-bytecode-test
    let error = |bytes: &[u8]| code(bytes).decode().unwrap_err().to_string();
    assert_eq!(
        error(&[0x11, 0x00]), // sipush, one byte short
        "ClassFormatError: truncated class file: 2 bytes needed at offset 1, 1 left"
    );
    assert_eq!(
        error(&[0xa7, 0x00, 0x02, 0xb1]), // goto into its own operand
        "ClassFormatError: the branch at 0 refers to 2, \
         which is not the start of an instruction"
    );
    assert_eq!(
        error(&[0xa8, 0x00, 0x03, 0xb1]),
        "ClassFormatError: the opcode 0xa8 at 0 is not allowed after Java 6"
    );
    assert_eq!(
        error(&[0xc4, 0x60]), // wide iadd
        "ClassFormatError: the opcode 0x60 at 1 cannot follow wide"
    );
    assert!(error(&[0xfe]).contains("is not an instruction")); // impdep1
    let unsorted = [0xab, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 2, 0, 0, 0, 5, 0, 0, 0, 12];
    let unsorted = [&unsorted[..], &[0, 0, 0, 1, 0, 0, 0, 12]].concat();
    assert!(error(&unsorted).contains("are not sorted"));
    // @end
}

#[test]
fn the_disassembly_of_main() {
    // @listing disassembly-test
    let class = parse("Shapes", "Shapes");
    let lines: Vec<String> = instructions(&class, "main")[..6]
        .iter()
        .map(|i| i.disassemble(&class).unwrap())
        .collect();
    assert_eq!(
        lines,
        [
            "    0: iconst_2",
            "    1: anewarray     #13                 // class Shape",
            "    4: dup",
            "    5: iconst_0",
            "    6: new           #15                 // class Square",
            "    9: dup",
        ]
    );
    // @end
}

#[test]
fn every_method_matches_javap() {
    // @listing javap-test
    let classes = [
        ("Hello", "Hello"),
        ("Lambdas", "Lambdas"),
        ("Lambdas", "Lambdas$Point"),
        ("Shapes", "Shape"),
        ("Shapes", "Polygon"),
        ("Shapes", "Square"),
        ("Shapes", "Circle"),
        ("Shapes", "Shapes"),
        ("Switches", "Switches"),
    ];
    let words = |line: &str| line.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut compared = 0;
    for (source, name) in classes {
        let path = compile(source, name);
        let class = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
        let mut ours = Vec::new();
        for code in class.methods.iter().filter_map(|m| m.code.as_ref()) {
            for i in code.decode().unwrap() {
                ours.extend(i.disassemble(&class).unwrap().lines().map(words));
            }
        }
        // javap -c prints each method's code after a "Code:" line, up to
        // the blank line that ends the method.
        let javap = Jdk::find().unwrap().javap_code(&path).unwrap();
        let javap = javap.trim_end().strip_suffix('}').unwrap(); // the class's
        let theirs: Vec<String> = javap
            .split("    Code:\n")
            .skip(1)
            .flat_map(|method| method.split("\n\n").next().unwrap().lines())
            .map(words)
            .collect();
        assert_eq!(ours, theirs, "{name}");
        compared += ours.len();
    }
    assert_eq!(compared, 254);
    // @end
}
