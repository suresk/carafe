//! The scoreboard of Chapter 24: `cargo test --test book_ch24`.
use carafe::{Jdk, Vm};
use std::collections::BTreeSet;
use std::path::Path;

// @listing score
/// How a corpus program did on carafe: whether its standard output matched
/// HotSpot's, and the classes, natives and instructions it took.
#[derive(Debug)]
struct Score {
    program: String,
    same: bool,
    classes: usize,
    natives: BTreeSet<String>,
    instructions: u64,
}

/// The standard input every program gets; only `ReadInput` reads it.
const INPUT: &[u8] = b"4\n5\n33\n";

/// Compiles `carafe/corpus/<program>.java` into a directory of its own, since
/// the corpus reuses names the book's programs have, and scores it.
fn score(jdk: &Jdk, program: &str) -> Score {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("corpus").join(program);
    jdk.javac(&corpus.join(format!("{program}.java")), &out).unwrap();
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("jdk");
    let mut vm = Vm::boot(jdk, &work, &out).unwrap();
    vm.stdin = INPUT.to_vec();
    let ran = vm.run_main(program);
    let options = ["-XX:-ShowCodeDetailsInExceptionMessages"];
    let theirs = jdk.java_input(&options, &out, program, INPUT).unwrap();
    let same = ran.is_ok() && String::from_utf8(vm.stdout.clone()).unwrap() == theirs;
    let (classes, instructions) = (vm.loaded.len(), vm.executed);
    Score {
        program: program.into(),
        same,
        classes,
        natives: vm.natives,
        instructions,
    }
}
// @end

#[test]
fn the_scoreboard() {
    // @listing scoreboard-test
    let jdk = Jdk::find().unwrap();
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut programs: Vec<String> = std::fs::read_dir(&corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "java"))
        .map(|path| path.file_stem().unwrap().to_string_lossy().into())
        .collect();
    programs.sort();
    let scores: Vec<Score> = programs.iter().map(|p| score(&jdk, p)).collect();
    let table: Vec<(&str, bool, usize, usize)> = scores
        .iter()
        .map(|s| (s.program.as_str(), s.same, s.classes, s.natives.len()))
        .collect();
    assert_eq!(
        table,
        [
            ("BigNumbers", true, 71, 15),
            ("Collections2", true, 131, 24),
            ("Enums", true, 129, 27),
            ("Exceptions", true, 91, 15),
            ("Factories", true, 91, 18),
            ("Format", false, 252, 42),
            ("Hello", true, 31, 5),
            ("HelpfulNpe", true, 49, 8),
            ("Modest", true, 108, 17),
            ("Numbers", true, 113, 26),
            ("PatternSwitch", true, 79, 16),
            ("ReadInput", true, 114, 24),
            ("Records", true, 61, 10),
            ("Streams", true, 226, 35),
            ("StringSwitch", true, 50, 10),
            ("Strings", true, 127, 24),
        ]
    );
    let passed: Vec<&Score> = scores.iter().filter(|s| s.same).collect();
    assert_eq!(passed.len(), 15);
    let natives: BTreeSet<&String> = passed.iter().flat_map(|s| &s.natives).collect();
    assert_eq!(natives.len(), 41);
    let busiest = passed.iter().max_by_key(|s| s.instructions).unwrap();
    assert_eq!(
        (busiest.program.as_str(), busiest.instructions),
        ("Strings", 481_316)
    );
    // @end
}
