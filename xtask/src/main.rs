//! `cargo xtask check 1` runs Chapter 1's tests (the chapter's examples and each of
//! its challenges) and says which pass. `cargo xtask check 1.3` runs one challenge,
//! and `cargo xtask check all` every chapter. Add `--crate <dir>` to run them against
//! your own crate. See `check.rs`.
//! `cargo xtask pool Hello.class` prints a class file's version and constant pool.

mod check;

use carafe::ClassFile;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["check", ref rest @ ..] => check::main(rest),
        ["pool", path] => match pool(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{path}: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!(
                "usage: cargo xtask check <chapter | chapter.challenge | all> \
                 [--crate <dir>]"
            );
            eprintln!("       cargo xtask pool <file.class>");
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
