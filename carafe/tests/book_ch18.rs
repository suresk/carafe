//! The examples in Chapter 18: `cargo test --test book_ch18`.
use carafe::{CallSite, Class, Jdk, Vm};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Compiles `java/<source>.java` with the pinned JDK and returns the path of
/// the class file for `class`, one of the classes the source declares.
fn compile(source: &str, class: &str) -> PathBuf {
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("java");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("classes");
    let jdk = Jdk::find().unwrap();
    jdk.javac(&java.join(format!("{source}.java")), &out).unwrap();
    out.join(format!("{class}.class"))
}

/// Runs `program` on carafe and on HotSpot, and returns carafe's VM, what it
/// printed, and what HotSpot printed.
fn both(program: &str) -> (Vm, String, String) {
    let app = compile(program, program).parent().unwrap().to_path_buf();
    let jdk = Jdk::find().unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(&jdk, &work, &app).unwrap();
    vm.run_main(program).unwrap();
    let ours = String::from_utf8(vm.stdout.clone()).unwrap();
    let theirs = jdk.java(&app, program).unwrap();
    (vm, ours, theirs)
}

// @listing lambda-classes
/// The lambda classes linked by call sites in `class`, in the order of their
/// names, which is the order carafe made them.
fn lambda_classes(class: &Class) -> Vec<Rc<Class>> {
    let sites = class.call_sites.borrow();
    let mut classes: Vec<Rc<Class>> = sites
        .values()
        .filter_map(|site| match &**site {
            CallSite::Lambda { class, .. } => Some(class.clone()),
            _ => None,
        })
        .collect();
    let number = |c: &Rc<Class>| {
        let name = &c.file.this_class;
        name[name.rfind('$').unwrap() + 1..].parse::<usize>().unwrap()
    };
    classes.sort_by_key(number);
    classes
}

/// The code of a class's first method, as `javap -c` prints it.
fn code(class: &Class) -> Vec<String> {
    let code = class.instructions(0).unwrap();
    code.iter().map(|i| i.disassemble(&class.file).unwrap()).collect()
}
// @end

#[test]
fn lambdas() {
    // @listing lambdas-test
    let (_, ours, theirs) = both("Functional");
    assert_eq!(ours, theirs);
    assert!(ours.contains("[fig, pear, kiwi, apple, banana]\n"));
    assert!(ours.ends_with("true\nrun\n[1, 3, 5, 7, 9]\nbottle carafe Mug\n"));
    // @end
}

#[test]
fn a_lambda_class() {
    // @listing lambda-class-test
    let (mut vm, _, _) = both("Functional");
    let functional = vm.load("Functional").unwrap();
    let classes = lambda_classes(&functional);
    let sum = &classes[2]; // BinaryOperator<Integer> sum = Integer::sum
    assert_eq!(sum.file.this_class, "Functional$$Lambda$2");
    assert_eq!(sum.file.interfaces, ["java/util/function/BinaryOperator"]);
    let mnemonics: Vec<String> =
        sum.instructions(0).unwrap().iter().map(|i| i.mnemonic()).collect();
    let unbox = ["aload", "checkcast", "invokevirtual"];
    let call_and_box = ["invokestatic", "invokestatic", "areturn"];
    assert_eq!(mnemonics, [&unbox[..], &unbox[..], &call_and_box[..]].concat());
    assert!(code(sum)[6].ends_with("// Method java/lang/Integer.sum:(II)I"));
    // Lambdas 10 and 11 are Comparator's own, made while sorting.
    let greeter = &classes[12]; // name -> greeting + ", " + name, which captures this
    assert_eq!(greeter.file.this_class, "Functional$$Lambda$14");
    assert_eq!(greeter.file.fields[0].descriptor, "LFunctional;");
    // @end
}
