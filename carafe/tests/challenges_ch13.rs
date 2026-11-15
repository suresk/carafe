//! Tests for the Chapter 13 challenges: `cargo test --test challenges_ch13`.
use carafe::{ClassPath, Jdk, Vm};
use std::path::{Path, PathBuf};

/// Compiles `java/<source>.java` and returns the path of the class file `class`.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

fn boot(app: &Path) -> Vm {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    Vm::boot(&Jdk::find().unwrap(), &work, app).unwrap()
}

// @challenge 1
#[test]
fn predict_the_answers() {
    let app = compile("Casts", "Casts").parent().unwrap().to_path_buf();
    let mut vm = boot(&app);
    let mut assignable = |s: &str, t: &str| {
        let (s, t) = (vm.load(s).unwrap(), vm.load(t).unwrap());
        vm.is_assignable(&s, &t).unwrap()
    };
    assert!(assignable("[[Ljava/lang/String;", "[[Ljava/lang/Object;"));
    assert!(assignable("[[Ljava/lang/String;", "[Ljava/io/Serializable;"));
    assert!(!assignable("[Ljava/lang/Object;", "[Ljava/lang/String;"));
    assert!(assignable("Ambidextrous", "Right"));
    assert!(!assignable("Left", "Right"));
    assert!(assignable("Left", "java/lang/Object"));
}

// @challenge 2
use carafe::illegal_access_message;

#[test]
fn illegal_access_completed() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch13-challenge-access");
    std::fs::create_dir_all(&dir).unwrap();
    for class in
        ["Loading", "Geometry", "Figure", "Labeled", "Rectangle", "Window", "Unused"]
    {
        std::fs::copy(compile("Loading", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    // Make Rectangle.width private, as Chapter 8's test does.
    let path = dir.join("Rectangle.class");
    let mut bytes = std::fs::read(&path).unwrap();
    let pool = carafe::ClassFile::parse(&bytes).unwrap().constant_pool;
    let width = carafe::Constant::Utf8("width".to_string());
    let (index, _) = pool.iter().find(|(_, c)| **c == width).unwrap();
    let at = bytes.windows(2).rposition(|w| w == index.to_be_bytes()).unwrap() - 2;
    bytes[at..at + 2].copy_from_slice(&2u16.to_be_bytes());
    std::fs::write(&path, bytes).unwrap();

    let mut vm = Vm::new();
    vm.class_path = ClassPath { dirs: vec![dir.clone()] };
    let loading = vm.load("Loading").unwrap();
    let error = vm.invoke(&loading, "windowArea", "(I)I", &[carafe::Value::Int(3)]);
    let carafe::Error::Exception { message: Some(message), .. } = error.unwrap_err()
    else {
        panic!("not an exception")
    };
    let (geometry, rectangle) =
        (vm.load("Geometry").unwrap(), vm.load("Rectangle").unwrap());
    let full = illegal_access_message(&message, &geometry, &rectangle);
    let Err(carafe::Error::Jdk(hotspot)) = Jdk::find().unwrap().java(&dir, "Loading")
    else {
        panic!("HotSpot ran it")
    };
    assert!(
        hotspot.contains(&format!("java.lang.IllegalAccessError: {full}\n")),
        "{full}"
    );
}

// @challenge 3
use carafe::conflicting_defaults;

#[test]
fn conflicting_default_methods() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch13-challenge-defaults");
    std::fs::create_dir_all(&dir).unwrap();
    for class in
        ["Casts", "Named", "Robot", "Person", "Left", "Right", "Ambidextrous"]
    {
        std::fs::copy(compile("Casts", class), dir.join(format!("{class}.class")))
            .unwrap();
    }
    let path = dir.join("Ambidextrous.class");
    let bytes = std::fs::read(&path).unwrap();
    let mut changed = bytes.clone();
    let at = bytes.windows(4).rposition(|w| w == b"side").unwrap();
    changed[at + 3] = b'x'; // Ambidextrous declares sidx, overriding neither default
    std::fs::write(&path, changed).unwrap();
    let mut vm = boot(&dir);
    let ambidextrous = vm.load("Ambidextrous").unwrap();
    let message = conflicting_defaults(&ambidextrous, "side", "()Ljava/lang/String;");
    assert_eq!(
        message.as_deref(),
        Some("Conflicting default methods: Left.side Right.side")
    );
    let robot = vm.load("Robot").unwrap();
    assert_eq!(
        conflicting_defaults(&robot, "greeting", "()Ljava/lang/String;"),
        None
    );
}
