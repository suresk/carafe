//! Tests for the Chapter 1 challenges: `cargo test --test challenges_ch01`.
use carafe::{ClassFile, ConstantPool, Jdk, Reader, decode_modified_utf8};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn bytes(source: &str, class: &str) -> Vec<u8> {
    std::fs::read(compile(source, class)).unwrap()
}

fn pool(source: &str, class: &str) -> ConstantPool {
    ClassFile::parse(&bytes(source, class)).unwrap().constant_pool
}

fn pool_of(name: &str) -> ConstantPool {
    pool(name, name)
}

/// Every class file the book's Java programs so far compile to.
const CLASSES: [(&str, &str); 4] = [
    ("Hello", "Hello"),
    ("Constants", "Constants"),
    ("Lambdas", "Lambdas"),
    ("Lambdas", "Lambdas$Point"),
];

// @challenge 2 5
use carafe::encode_modified_utf8;

// @challenge 3
use carafe::check_pool;

// @challenge 4
use carafe::tag_counts;

// @challenge 5
use carafe::write_pool;

// @challenge 1
#[test]
fn predict_the_pool() {
    // A Long 1, a Utf8 "x" and a Double 2.0, count first.
    let bytes = [
        0, 6, // constant_pool_count
        5, 0, 0, 0, 0, 0, 0, 0, 1, // #1: Long 1 (and #2)
        1, 0, 1, b'x', // #3: Utf8 "x"
        6, 0x40, 0, 0, 0, 0, 0, 0, 0, // #4: Double 2.0 (and #5)
    ];
    let mut r = Reader::new(&bytes);
    let pool = ConstantPool::read(&mut r).unwrap();
    assert_eq!(pool.count(), 6);
    assert_eq!(pool.iter().map(|(i, _)| i).collect::<Vec<_>>(), [1, 3, 4]);
    assert_eq!(r.offset(), 24);

    // "€uro" is six bytes of modified UTF-8: three for the euro sign.
    let euro = [0xe2, 0x82, 0xac, b'u', b'r', b'o'];
    assert_eq!(
        String::from_utf16(&decode_modified_utf8(&euro).unwrap()).unwrap(),
        "€uro"
    );

    // Seventeen of Hello's 28 entries are Utf8.
    let hello = pool_of("Hello");
    let utf8 = hello.iter().filter(|(_, c)| matches!(c, carafe::Constant::Utf8(_)));
    assert_eq!(utf8.count(), 17);
}

// @challenge 2
#[test]
fn encode_is_the_inverse_of_decode() {
    for unit in 0..=u16::MAX {
        let bytes = encode_modified_utf8(&[unit]);
        assert_eq!(decode_modified_utf8(&bytes).unwrap(), [unit], "{unit:#06x}");
    }
    let units: Vec<u16> = "A\0é€😀".encode_utf16().collect();
    assert_eq!(
        encode_modified_utf8(&units),
        [
            0x41, 0xc0, 0x80, 0xc3, 0xa9, 0xe2, 0x82, 0xac, 0xed, 0xa0, 0xbd, 0xed,
            0xb8, 0x80
        ]
    );
}

// @challenge 3
#[test]
fn javac_output_passes_the_check() {
    for (source, class) in CLASSES {
        check_pool(&pool(source, class)).unwrap();
    }
}

// @challenge 3
#[test]
fn bad_references_are_caught() {
    let check = |bytes: &[u8]| {
        let pool = ConstantPool::read(&mut Reader::new(bytes)).unwrap();
        check_pool(&pool).map_err(|e| e.to_string())
    };
    // #1 = Class #2, but #2 is an Integer.
    assert_eq!(
        check(&[0, 3, 7, 0, 2, 3, 0, 0, 0, 5]),
        Err("ClassFormatError: #2 is Integer(5), not a Utf8".to_string())
    );
    // #1 = Class #3, past the end.
    assert_eq!(
        check(&[0, 3, 7, 0, 3, 1, 0, 1, b'x']),
        Err("ClassFormatError: #3 is not a usable constant pool index".to_string())
    );
    // #1 = MethodHandle of kind 6 (invokeStatic) that refers to #2, a Utf8.
    assert_eq!(
        check(&[0, 3, 15, 6, 0, 2, 1, 0, 1, b'x']),
        Err("ClassFormatError: #1: kind 6 cannot refer to Utf8(\"x\")".to_string())
    );
    // #1 = String #2, a Utf8: fine.
    assert_eq!(check(&[0, 3, 8, 0, 2, 1, 0, 1, b'x']), Ok(()));
}

// @challenge 4
#[test]
fn tags_in_the_wild() {
    let hello = tag_counts(&pool_of("Hello"));
    let tags: Vec<_> = hello.into_iter().collect();
    assert_eq!(
        tags,
        [
            ("Class", 4),
            ("Fieldref", 1),
            ("Methodref", 2),
            ("NameAndType", 3),
            ("String", 1),
            ("Utf8", 17)
        ]
    );
    let lambdas = tag_counts(&pool_of("Lambdas"));
    assert_eq!(lambdas["InvokeDynamic"], 2);
    assert_eq!(lambdas["MethodHandle"], 3);
    assert_eq!(lambdas["MethodType"], 1);
    assert_eq!(lambdas["InterfaceMethodref"], 1);
    let point = tag_counts(&pool("Lambdas", "Lambdas$Point"));
    assert_eq!(point["InvokeDynamic"], 3);
    assert_eq!(point.get("MethodType"), None);
}

// @challenge 5
#[test]
fn the_pool_writes_back_byte_for_byte() {
    for (source, class) in CLASSES {
        let bytes = bytes(source, class);
        let mut r = Reader::new(&bytes[8..]);
        let pool = ConstantPool::read(&mut r).unwrap();
        assert_eq!(write_pool(&pool), &bytes[8..8 + r.offset()], "{class}");
    }
}
