//! The examples in Chapter 1: `cargo test --test book_ch01`.
use carafe::{ClassFile, Constant, Jdk, MemberRef, Reader, decode_modified_utf8};
use std::path::{Path, PathBuf};

// @listing compile
/// Compiles `java/<name>.java` with the pinned JDK and returns the path of
/// `<name>.class`.
fn compile(name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&source.join(format!("{name}.java")), &out).unwrap();
    out.join(format!("{name}.class"))
}

/// Compiles `java/<name>.java` and parses the class file.
fn parse(name: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile(name)).unwrap()).unwrap()
}
// @end

#[test]
fn hello_starts_with_magic_and_version() {
    // @listing header
    let bytes = std::fs::read(compile("Hello")).unwrap();
    assert_eq!(bytes.len(), 417);
    assert_eq!(bytes[..10], [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 0x45, 0, 0x1d]);

    let class = ClassFile::parse(&bytes).unwrap();
    assert_eq!((class.major_version, class.minor_version), (69, 0));
    assert_eq!(class.constant_pool.count(), 29);
    // @end
}

#[test]
fn reads_are_checked() {
    // @listing reader-test
    let mut r = Reader::new(&[0xca, 0xfe, 0xba, 0xbe, 0x00, 0x2a]);
    assert_eq!(r.u4().unwrap(), 0xcafe_babe);
    assert_eq!(r.u1().unwrap(), 0);
    let Err(e) = r.u2() else { panic!("read past the end") };
    assert_eq!(
        e.to_string(),
        "ClassFormatError: truncated class file: 2 bytes needed at offset 5, 1 left"
    );
    // @end
}

#[test]
fn files_that_are_not_class_files() {
    // @listing not-ours
    let error = |bytes: &[u8]| ClassFile::parse(bytes).unwrap_err().to_string();
    assert_eq!(
        error(b"PK\x03\x04\x14\x00\x00\x00"),
        "ClassFormatError: the file starts with 0x504b0304, not 0xcafebabe"
    );
    assert_eq!(
        error(&[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 70]),
        "UnsupportedClassVersionError: class file version 70.0 is outside \
         the versions carafe runs, 51.0 to 69.0"
    );
    assert_eq!(
        error(&[0xca, 0xfe, 0xba, 0xbe, 0xff, 0xff, 0, 69]),
        "UnsupportedClassVersionError: class file version 69.65535 uses \
         preview features"
    );
    assert_eq!(
        error(&[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 69, 0, 3, 2]),
        "ClassFormatError: unknown constant pool tag 2 at offset 10"
    );
    // @end
}

#[test]
fn system_out() {
    // @listing system-out
    let pool = parse("Hello").constant_pool;
    assert_eq!(
        pool.get(7).unwrap(),
        &Constant::Fieldref { class: 8, name_and_type: 9 }
    );
    let out = MemberRef {
        class: "java/lang/System",
        name: "out",
        descriptor: "Ljava/io/PrintStream;",
    };
    assert_eq!(pool.member(7).unwrap(), out);
    assert_eq!(pool.class_name(21).unwrap(), "Hello");
    // @end
}

#[test]
fn indices_are_checked() {
    // @listing wrong-kind
    let pool = parse("Hello").constant_pool;
    let error = |e: carafe::Error| e.to_string();
    assert_eq!(
        error(pool.class_name(4).unwrap_err()),
        "ClassFormatError: #4 is Utf8(\"java/lang/Object\"), not a Class"
    );
    assert_eq!(
        error(pool.member(13).unwrap_err()),
        "ClassFormatError: #13 is String { utf8: 14 }, not a field or method reference"
    );
    assert!(pool.get(0).is_err());
    assert!(pool.get(29).is_err());
    // @end
}

#[test]
fn modified_utf8() {
    // @listing utf8-test
    let units = decode_modified_utf8(&[
        0x41, // A
        0xc0, 0x80, // U+0000, in two bytes
        0xc3, 0xa9, // é, U+00E9
        0xe2, 0x82, 0xac, // €, U+20AC
        0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80, // 😀, U+1F600, as two surrogates
    ])
    .unwrap();
    assert_eq!(units, [0x41, 0x0000, 0x00e9, 0x20ac, 0xd83d, 0xde00]);
    assert_eq!(String::from_utf16(&units).unwrap(), "A\0é€😀");

    assert!(decode_modified_utf8(&[0x00]).is_err()); // a zero byte
    assert!(decode_modified_utf8(&[0xc3]).is_err()); // a group cut short
    assert!(decode_modified_utf8(&[0xf0, 0x9f, 0x98, 0x80]).is_err()); // UTF-8's 😀
    // @end
}

#[test]
fn the_other_constants() {
    // @listing constants-test
    let pool = parse("Constants").constant_pool;
    assert_eq!(pool.count(), 31);
    assert_eq!(pool.get(7).unwrap(), &Constant::Long(10_000_000_000));
    assert!(pool.get(8).is_err()); // the Long's second slot
    assert_eq!(pool.get(9).unwrap(), &Constant::Double(0.5));
    assert_eq!(pool.get(11).unwrap(), &Constant::Integer(1_000_000));
    assert_eq!(pool.utf8(13).unwrap(), "nul\0 é € 😀");
    assert_eq!(pool.get(15).unwrap(), &Constant::Utf16(vec![0xd800]));
    assert!(pool.utf8(15).is_err());
    // @end
}

#[test]
fn the_pool_as_carafe_prints_it() {
    // @listing display-test
    let printed = parse("Hello").constant_pool.to_string();
    let first: Vec<&str> = printed.lines().take(7).collect();
    assert_eq!(
        first,
        [
            "  #1 = Methodref   #2.#3   // java/lang/Object.\"<init>\":()V",
            "  #2 = Class       #4      // java/lang/Object",
            "  #3 = NameAndType #5:#6   // \"<init>\":()V",
            "  #4 = Utf8        java/lang/Object",
            "  #5 = Utf8        <init>",
            "  #6 = Utf8        ()V",
            "  #7 = Fieldref    #8.#9   // java/lang/System.out:Ljava/io/PrintStream;",
        ]
    );
    // @end
}

#[test]
fn the_pool_matches_javap() {
    // @listing javap-test
    let path = compile("Hello");
    let javap = Jdk::find().unwrap().javap(&path).unwrap();
    let section = javap.split("Constant pool:\n").nth(1).unwrap();
    let section = section.split("\n{").next().unwrap();
    let words = |text: &str| -> Vec<String> {
        let words =
            |line: &str| line.split_whitespace().collect::<Vec<_>>().join(" ");
        text.lines().map(words).collect()
    };
    let class = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(words(&class.constant_pool.to_string()), words(section));
    // @end
}
