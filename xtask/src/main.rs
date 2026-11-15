//! `cargo xtask check 1` runs Chapter 1's tests (the chapter's examples and each of
//! its challenges) and says which pass. `cargo xtask check 1.3` runs one challenge,
//! and `cargo xtask check all` every chapter. Add `--crate <dir>` to run them against
//! your own crate. See `check.rs`.
//! `cargo xtask pool Hello.class` prints a class file's version and constant pool.
//! `cargo xtask code Hello.class` prints the bytecode of each method.
//! `cargo xtask run <dir> Hello` runs `Hello.main` on carafe, with `<dir>` as the
//! class path after the JDK's classes.

mod check;

use carafe::{ClassFile, Jdk, Vm};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["check", ref rest @ ..] => check::main(rest),
        ["pool", path] => report(path, pool(path)),
        ["code", path] => report(path, code(path)),
        ["run", dir, class] => run(dir, class),
        _ => {
            eprintln!(
                "usage: cargo xtask check <chapter | chapter.challenge | all> \
                 [--crate <dir>]"
            );
            eprintln!("       cargo xtask pool <file.class>");
            eprintln!("       cargo xtask code <file.class>");
            eprintln!("       cargo xtask run <class directory> <main class>");
            ExitCode::from(2)
        }
    }
}

// @listing pool
/// Prints the version and the constant pool of the class file at `path`.
fn pool(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let class = ClassFile::parse(&std::fs::read(path)?)?;
    println!("version {}.{}", class.major_version, class.minor_version);
    print!("{}", class.constant_pool);
    Ok(())
}
// @end

// @listing code
/// Prints each method of the class file at `path` and its instructions.
fn code(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let class = ClassFile::parse(&std::fs::read(path)?)?;
    for method in &class.methods {
        println!("{}{}", method.name, method.descriptor);
        if let Some(code) = &method.code {
            for i in code.decode()? {
                println!("{}", i.disassemble(&class)?);
            }
        }
    }
    Ok(())
}
// @end

fn report(path: &str, result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{path}: {e}");
            ExitCode::FAILURE
        }
    }
}

// @listing run
/// Runs `class`'s `main` on carafe, prints what it wrote, and reports an
/// uncaught exception as `java` does.
fn run(dir: &str, class: &str) -> ExitCode {
    let work = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/carafe");
    let booted = Jdk::find().and_then(|jdk| Vm::boot(&jdk, &work, Path::new(dir)));
    let mut vm = match booted {
        Ok(vm) => vm,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    // @hide
    // The program reads standard input as it asks for it.
    vm.stdin_source = Some(Box::new(std::io::stdin()));
    // @show
    let result = vm.run_main(class);
    std::io::stdout().write_all(&vm.stdout).expect("stdout");
    std::io::stderr().write_all(&vm.stderr).expect("stderr");
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // @hide
        // The report is in vm.stderr, written above.
        Err(carafe::Error::Thrown { .. }) => ExitCode::FAILURE,
        // @show
        Err(e) => {
            eprintln!("Exception in thread \"main\" {e}");
            ExitCode::FAILURE
        }
    }
}
// @end
