//! The examples in Chapter 2: `cargo test --test book_ch02`.
use carafe::{AccessFlags, ClassFile, FieldType, Flagged, Jdk, MethodDescriptor};
use std::path::{Path, PathBuf};

// @listing compile
/// Compiles `java/<source>.java` with the pinned JDK and returns the path of
/// the class file for `class`, one of the classes the source declares.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

/// Parses the class file for `class`, from `java/Shapes.java`.
fn shapes(class: &str) -> ClassFile {
    ClassFile::parse(&std::fs::read(compile("Shapes", class)).unwrap()).unwrap()
}
// @end

#[test]
fn flags_depend_on_what_they_flag() {
    // @listing flags-test
    let flags = AccessFlags(0x0021);
    assert!(flags.has(AccessFlags::PUBLIC | AccessFlags::STATIC));
    assert_eq!(flags.describe(Flagged::Class), "(0x0021) ACC_PUBLIC, ACC_SUPER");
    let method = flags.describe(Flagged::Method);
    assert_eq!(method, "(0x0021) ACC_PUBLIC, ACC_SYNCHRONIZED");
    assert_eq!(AccessFlags(0x0040).describe(Flagged::Field), "(0x0040) ACC_VOLATILE");
    assert_eq!(AccessFlags(0).describe(Flagged::Method), "(0x0000)");
    // @end
}

#[test]
fn descriptors() {
    // @listing descriptor-test
    let string = FieldType::Object("java/lang/String".to_string());
    let args = FieldType::parse("[Ljava/lang/String;").unwrap();
    assert_eq!(args, FieldType::Array(Box::new(string)));
    assert_eq!(args.to_string(), "java.lang.String[]");

    let scale = MethodDescriptor::parse("(DJ[I)D").unwrap();
    let params: Vec<String> = scale.params.iter().map(|p| p.to_string()).collect();
    assert_eq!(params, ["double", "long", "int[]"]);
    assert_eq!(scale.ret, Some(FieldType::Double));
    assert_eq!(scale.param_slots(), 5);
    assert_eq!(MethodDescriptor::parse("()V").unwrap().ret, None);

    let error = |text: &str| MethodDescriptor::parse(text).unwrap_err().to_string();
    assert_eq!(error("(I;)V"), "ClassFormatError: malformed descriptor \"(I;)V\"");
    assert!(MethodDescriptor::parse("(Ljava.lang.String;)V").is_err()); // dots
    assert!(MethodDescriptor::parse("(I)").is_err()); // no return type
    assert!(MethodDescriptor::parse("()VV").is_err()); // more after it
    assert!(FieldType::parse("V").is_err()); // void is not a field type
    // @end
}

#[test]
fn five_classes() {
    // @listing shapes-test
    let square = shapes("Square");
    assert_eq!(square.this_class, "Square");
    assert_eq!(square.super_class.as_deref(), Some("Polygon"));
    assert_eq!(shapes("Polygon").interfaces, ["Shape"]);
    assert_eq!(shapes("Shape").super_class.as_deref(), Some("java/lang/Object"));

    let methods = |class: &ClassFile| -> Vec<String> {
        let method = |m: &carafe::Method| format!("{}{}", m.name, m.descriptor);
        class.methods.iter().map(method).collect()
    };
    let class = shapes("Shapes");
    assert_eq!(
        methods(&class),
        ["<init>()V", "ticks()J", "scale(DJ[I)D", "main([Ljava/lang/String;)V"]
    );
    assert_eq!(methods(&shapes("Shape")), ["area()D", "name()Ljava/lang/String;"]);
    let ticks = class.method("ticks", "()J").unwrap();
    assert!(ticks.code.is_none()); // a native method has no bytecode
    assert_eq!(class.method("scale", "(DJ[I)D").unwrap().arg_slots(), 5);
    assert_eq!(square.method("<init>", "(D)V").unwrap().arg_slots(), 3);
    // @end
}

#[test]
fn the_constant_value_of_a_field() {
    // @listing constant-value-test
    let polygon = shapes("Polygon");
    let [max, sides] = &polygon.fields[..] else { panic!("two fields") };
    assert_eq!((max.name.as_str(), &max.ty), ("MAX_SIDES", &FieldType::Int));
    assert_eq!(
        max.access_flags.describe(Flagged::Field),
        "(0x0018) ACC_STATIC, ACC_FINAL"
    );
    let index = max.constant_value.unwrap();
    assert_eq!(
        polygon.constant_pool.get(index).unwrap(),
        &carafe::Constant::Integer(12)
    );
    assert_eq!(sides.constant_value, None); // set by the constructor
    // @end
}

#[test]
fn the_code_of_main() {
    // @listing code-test
    let hello = std::fs::read(compile("Hello", "Hello")).unwrap();
    let hello = ClassFile::parse(&hello).unwrap();
    let init = hello.method("<init>", "()V").unwrap().code.as_ref().unwrap();
    assert_eq!((init.max_stack, init.max_locals), (1, 1));
    assert_eq!(init.bytecode, [0x2a, 0xb7, 0x00, 0x01, 0xb1]);

    let class = shapes("Shapes");
    let main = class.method("main", "([Ljava/lang/String;)V").unwrap();
    let code = main.code.as_ref().unwrap();
    assert_eq!((code.max_stack, code.max_locals), (7, 8));
    assert_eq!(code.bytecode.len(), 89);
    assert_eq!(code.bytecode[..6], [0x05, 0xbd, 0x00, 0x0d, 0x59, 0x03]);
    assert_eq!(code.bytecode.last(), Some(&0xb1)); // return
    assert!(code.handlers.is_empty());
    let names: Vec<&str> = code.attributes.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["LineNumberTable", "StackMapTable"]);
    // @end
}

#[test]
fn damaged_class_files() {
    // @listing damaged-test
    let hello = std::fs::read(compile("Hello", "Hello")).unwrap();
    let error = |bytes: &[u8]| ClassFile::parse(bytes).unwrap_err().to_string();

    let cut = &hello[..hello.len() - 1];
    assert_eq!(
        error(cut),
        "ClassFormatError: truncated class file: 2 bytes needed at offset 415, 1 left"
    );
    let longer = [&hello[..], &[0, 0, 0]].concat();
    assert_eq!(
        error(&longer),
        "ClassFormatError: the class file of Hello has 3 bytes after its last attribute"
    );

    // Rename the pool's one "Code" to "Cade", which no JVM recognizes.
    let mut renamed = hello.clone();
    let at = hello.windows(4).position(|w| w == b"Code").unwrap();
    renamed[at + 1] = b'a';
    assert_eq!(
        error(&renamed),
        "ClassFormatError: the method <init>()V has no Code attribute"
    );
    // @end
}

#[test]
fn the_members_match_javap() {
    // @listing javap-test
    let mut compared = 0;
    for name in ["Shape", "Polygon", "Square", "Circle", "Shapes"] {
        let path = compile("Shapes", name);
        let class = ClassFile::parse(&std::fs::read(&path).unwrap()).unwrap();
        let mut ours = vec![
            format!("flags: {}", class.access_flags.describe(Flagged::Class)),
            format!(
                "interfaces: {}, fields: {}, methods: {}, attributes: {}",
                class.interfaces.len(),
                class.fields.len(),
                class.methods.len(),
                class.attributes.len()
            ),
        ];
        for field in &class.fields {
            ours.push(format!("descriptor: {}", field.descriptor));
            ours.push(format!(
                "flags: {}",
                field.access_flags.describe(Flagged::Field)
            ));
        }
        for method in &class.methods {
            ours.push(format!("descriptor: {}", method.descriptor));
            ours.push(format!(
                "flags: {}",
                method.access_flags.describe(Flagged::Method)
            ));
            if let Some(code) = &method.code {
                let (stack, locals) = (code.max_stack, code.max_locals);
                let this = usize::from(!method.access_flags.has(AccessFlags::STATIC));
                // javap counts arguments, not slots
                let args = this + method.ty.params.len();
                ours.push(format!(
                    "stack={stack}, locals={locals}, args_size={args}"
                ));
            }
        }
        let javap = Jdk::find().unwrap().javap(&path).unwrap();
        let prefixes = ["flags:", "interfaces:", "descriptor:", "stack="];
        let theirs: Vec<&str> = javap
            .lines()
            .map(str::trim)
            .filter(|line| prefixes.iter().any(|p| line.starts_with(p)))
            .collect();
        assert_eq!(ours, theirs, "{name}");
        compared += ours.len();
    }
    assert_eq!(compared, 59);
    // @end
}
