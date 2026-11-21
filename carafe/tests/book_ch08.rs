//! The examples in Chapter 8: `cargo test --test book_ch08`.
use carafe::{ClassFile, ClassPath, Constant, Jdk, Ref, Value, Vm};
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
const CLASSES: [&str; 7] =
    ["Loading", "Geometry", "Figure", "Labeled", "Rectangle", "Window", "Unused"];

/// A directory of its own holding the class files of `Loading.java`, which
/// a test may change without affecting other tests.
fn class_dir(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch08").join(test);
    std::fs::create_dir_all(&dir).unwrap();
    for class in CLASSES {
        std::fs::copy(compile("Loading", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    dir
}

/// Loads `Loading` through `vm`'s class path and runs its `main`, as `java
/// Loading` does.
fn run(vm: &mut Vm) -> carafe::Result<Option<Value>> {
    let loading = vm.load("Loading")?;
    vm.invoke(&loading, "main", "([Ljava/lang/String;)V", &[Value::Ref(Ref::NULL)])
}

fn vm(dir: &Path) -> Vm {
    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir.to_path_buf()] };
    vm
}
// @end

#[test]
fn loading_on_demand() {
    // @listing order-test
    let dir = class_dir("order");
    let mut vm = vm(&dir);
    run(&mut vm).unwrap();
    assert_eq!(
        vm.loaded,
        ["Loading", "Labeled", "Figure", "Rectangle", "Window", "Geometry"]
    );

    // HotSpot loads the same classes in the same order.
    let jdk = Jdk::find().unwrap();
    let log = jdk.java_with(&["-Xlog:class+load"], &dir, "Loading").unwrap();
    let theirs: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("source: file:"))
        .map(|line| line.split_whitespace().nth(1).unwrap())
        .collect();
    assert_eq!(vm.loaded, theirs);
    // @end
}

#[test]
fn subclass_layout() {
    // @listing layout-test
    let mut vm = vm(&class_dir("layout"));
    let window = vm.load("Window").unwrap();
    let fields: Vec<(&str, u32)> =
        window.layout.fields.iter().map(|f| (f.name.as_str(), f.offset)).collect();
    assert_eq!(fields, [("corners", 8), ("width", 12), ("height", 16)]);
    let names = |c: &carafe::Class| c.file.this_class.clone();
    let rectangle = window.superclass.as_ref().unwrap();
    assert_eq!(names(rectangle), "Rectangle");
    assert_eq!(names(&rectangle.interfaces[0]), "Labeled");
    assert!(rectangle.superclass.as_ref().unwrap().superclass.is_none()); // Figure

    let loading = vm.load("Loading").unwrap();
    let area = vm.invoke(&loading, "windowArea", "(I)I", &[Value::Int(3)]).unwrap();
    assert_eq!(area, Some(Value::Int(9)));
    // @end
}

// @listing breakers
/// Replaces every occurrence of `from` in the class file `class` in `dir`.
fn patch(dir: &Path, class: &str, from: &[u8], to: &[u8]) {
    let path = dir.join(format!("{class}.class"));
    let mut bytes = std::fs::read(&path).unwrap();
    let mut at = 0;
    while let Some(found) = bytes[at..].windows(from.len()).position(|w| w == from) {
        at += found;
        bytes[at..at + from.len()].copy_from_slice(to);
        at += from.len();
    }
    std::fs::write(path, bytes).unwrap();
}

/// Sets the access flags of the field or method `name` of `class` in `dir`.
fn set_flags(dir: &Path, class: &str, name: &str, flags: u16) {
    let path = dir.join(format!("{class}.class"));
    let mut bytes = std::fs::read(&path).unwrap();
    let pool = ClassFile::parse(&bytes).unwrap().constant_pool;
    let utf8 = Constant::Utf8(name.to_string());
    let (index, _) = pool.iter().find(|(_, c)| **c == utf8).unwrap();
    // A member is flags, name, descriptor; the last such name is the member's.
    let at = bytes.windows(2).rposition(|w| w == index.to_be_bytes()).unwrap() - 2;
    bytes[at..at + 2].copy_from_slice(&flags.to_be_bytes());
    std::fs::write(path, bytes).unwrap();
}

/// carafe's error from running `Loading`, and the exception HotSpot reports.
fn both(dir: &Path) -> (String, String) {
    let ours = run(&mut vm(dir)).unwrap_err().to_string();
    let Err(carafe::Error::Jdk(stderr)) = Jdk::find().unwrap().java(dir, "Loading")
    else {
        panic!("HotSpot ran it")
    };
    let line = stderr.lines().find(|l| l.contains("java.lang.")).unwrap();
    let theirs = &line[line.find("java.lang.").unwrap()..];
    (ours, theirs.to_string())
}
// @end

#[test]
fn resolution_failures() {
    // @listing failures-test
    let dir = class_dir("missing");
    std::fs::remove_file(dir.join("Geometry.class")).unwrap();
    let (ours, theirs) = both(&dir);
    assert_eq!(ours, "java.lang.NoClassDefFoundError: Geometry");
    assert_eq!(ours, theirs);

    let dir = class_dir("no-method");
    patch(&dir, "Geometry", b"area", b"arex");
    let (ours, theirs) = both(&dir);
    assert_eq!(ours, "java.lang.NoSuchMethodError: 'int Geometry.area(Rectangle)'");
    assert_eq!(ours, theirs);

    let dir = class_dir("no-field");
    patch(&dir, "Rectangle", b"width", b"widtx");
    let (ours, theirs) = both(&dir);
    assert_eq!(
        ours,
        "java.lang.NoSuchFieldError: \
         Class Rectangle does not have member field 'int width'"
    );
    assert_eq!(ours, theirs);
    // @end
}

#[test]
fn incompatible_changes() {
    // @listing incompatible-test
    let dir = class_dir("static-field");
    set_flags(&dir, "Rectangle", "width", 0x0008); // ACC_STATIC
    let (ours, theirs) = both(&dir);
    assert_eq!(
        ours,
        "java.lang.IncompatibleClassChangeError: \
                      Expected non-static field Rectangle.width"
    );
    assert_eq!(ours, theirs);

    let dir = class_dir("private-field");
    set_flags(&dir, "Rectangle", "width", 0x0002); // ACC_PRIVATE
    let (ours, theirs) = both(&dir);
    assert_eq!(
        ours,
        "java.lang.IllegalAccessError: \
                      class Geometry tried to access private field Rectangle.width"
    );
    assert!(theirs.starts_with(&ours)); // HotSpot adds the module and loader

    let dir = class_dir("circular");
    patch(&dir, "Rectangle", b"Figure", b"Window"); // Rectangle extends Window
    let (ours, theirs) = both(&dir);
    assert_eq!(ours, "java.lang.ClassCircularityError: Rectangle");
    assert_eq!(ours, theirs);
    // @end
}
