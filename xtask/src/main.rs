//! `cargo xtask check 1` runs Chapter 1's tests (the chapter's examples and each of
//! its challenges) and says which pass. `cargo xtask check 1.3` runs one challenge,
//! and `cargo xtask check all` every chapter. Add `--crate <dir>` to run them against
//! your own crate. See `check.rs`.
//! `cargo xtask pool Hello.class` prints a class file's version and constant pool.
//! `cargo xtask code Hello.class` prints the bytecode of each method.

mod check;

use carafe::ClassFile;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["check", ref rest @ ..] => check::main(rest),
        ["pool", path] => report(path, pool(path)),
        ["code", path] => report(path, code(path)),
        _ => {
            eprintln!(
                "usage: cargo xtask check <chapter | chapter.challenge | all> \
                 [--crate <dir>]"
            );
            eprintln!("       cargo xtask pool <file.class>");
            eprintln!("       cargo xtask code <file.class>");
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
