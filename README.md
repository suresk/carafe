# carafe

carafe is a Java virtual machine written in safe Rust. It reads the class files `javac`
writes and runs them on the real JDK class library: `String`, `StringBuilder`, `ArrayList`,
`HashMap`, `BigInteger`, records, enums, lambdas, streams, pattern-matching `switch` and
exceptions. It has no dependencies beyond the standard library.

It is the code for the book
[*Build a JVM in Rust*](https://uresk.net/workbooks/build-a-jvm-in-rust) by Spencer Uresk.
The book builds carafe one chapter at a time, and every listing in it is printed from this
repository, so the code here is the code on the page.

carafe is for learning. It interprets every method, never frees memory and runs one thread,
and it is not a compatible implementation of the Java SE specification. It is tested
against HotSpot: the programs in `carafe/corpus` print exactly what HotSpot prints, apart
from `Format`, which uses the part of the library the book leaves out.

## What is here

| Chapter | | Main files |
|---|---|---|
| 1 | A class file is a stream of bytes | `src/reader.rs`, `src/utf8.rs`, `src/pool.rs`, `src/class_file.rs`, `src/jdk.rs`, `src/error.rs` |
| 2 | Fields, methods and attributes | `src/attribute.rs`, `src/descriptor.rs`, `src/flags.rs`, `src/member.rs` |
| 3 | Reading bytecode | `src/opcode.rs`, `src/bytecode.rs`, `src/disassemble.rs` |
| 4 | Frames, the operand stack and int | `src/class.rs`, `src/frame.rs`, `src/value.rs`, `src/interpreter.rs` |
| 5 | Calls and returns | `src/interpreter.rs` |
| 6 | Long, float, double and the rules of Java arithmetic | `src/numeric.rs` |
| 7 | An object is bytes | `src/heap.rs` |
| 8 | Loading classes | `src/loader.rs`, `src/resolve.rs` |
| 9 | Initialization | `src/init.rs` |
| 10 | Strings, the shims and Hello, world | `src/boot.rs`, `src/strings.rs`, `src/natives.rs`, `shim/java/lang/System.java`, `shim/java/io/PrintStream.java` |
| 11 | Virtual dispatch | `src/interpreter.rs`, `src/resolve.rs` |
| 12 | Arrays | `src/arrays.rs`, `src/mirror.rs` |
| 13 | Interfaces and casts | `src/casts.rs` |
| 14 | Natives, Unsafe and the shape of an int | `src/lang.rs`, `src/unsafe_access.rs` |
| 15 | Class objects, hash codes and HashMap | `src/mirror.rs`, `src/lang.rs` |
| 16 | Exceptions | `src/exceptions.rs` |
| 17 | Call sites and string concatenation | `src/dynamic.rs` |
| 18 | Lambdas | `src/lambda.rs` |
| 19 | Records, enums and pattern switch | `src/records.rs`, `src/switches.rs` |
| 20 | Just enough reflection | `src/reflection.rs`, `shim/java/lang/LangAccess.java` |
| 21 | Streams | `src/reflection.rs`, `src/unsafe_access.rs` |
| 22 | A main thread and standard input | `src/threads.rs` |
| 23 | How fast is carafe? | `src/cache.rs` |
| 24 | The scoreboard | `corpus/` |

Paths are inside `carafe/`. The Java programs the tests run are in `carafe/java`; the book
prints only the parts it discusses, so the full text of each is here. The commit history
has one commit per chapter, but the book is printed from the latest code: check out `main`.

## What you need

- Rust 1.99 or later (`rust-toolchain.toml` pins it).
- **Eclipse Temurin 25.0.5+8, exactly.** The tests compile the Java programs in
  `carafe/java/` with its `javac`, and the counts the book quotes are true of that release.
  Point `CARAFE_JDK` at its home directory (or `JAVA_HOME`, which is used when `CARAFE_JDK`
  isn't set). With SDKMAN: `sdk install java 25.0.5.0+8-tem`, then
  `export CARAFE_JDK=~/.sdkman/candidates/java/25.0.5.0+8-tem`. The tests refuse any other JDK
  and say why.

## Commands

```text
$ cargo test                               # every chapter's examples and challenges
$ cargo xtask pool Hello.class             # a class file's version and constant pool
$ cargo xtask code Hello.class             # the bytecode of each method
$ cargo xtask run target/tmp/classes Hello # run Hello.main on carafe
$ cargo xtask check 1                      # Chapter 1's examples, and each of its challenges
$ cargo xtask check 1.3                    # one challenge, with the full output
```

## Following the book

The repository is a Cargo workspace with two crates. `carafe/` is the library and `xtask/`
holds the tools above. Each chapter has two test files in `carafe/tests/`:

- `book_chNN.rs` holds the examples the chapter prints. Every number the text quotes is an
  assertion there.
- `challenges_chNN.rs` checks the end-of-chapter challenges. The solutions are in
  `carafe/src/challenges/chNN.rs`, so don't open that directory until you have tried them.

## Checking your own code

If you are writing carafe yourself as you read, the same command that checks a chapter here
runs its tests against your code. Clone this repository next to your workspace under another
name, so the two aren't confused, and point the command at your workspace:

```text
$ git clone https://github.com/suresk/carafe carafe-reference
$ cd carafe-reference
$ cargo xtask check 1 --crate ../carafe-workspace
```

Each challenge is compiled on its own, so the ones you haven't written don't get in the way
of the ones you have. Nothing is written into your workspace: the tests are copied to
`target/check` here and built against your crate, along with the Java programs in
`carafe/java`. To leave `--crate` out, set it once: `export CARAFE_CRATE=../carafe-workspace`.

The `// @listing` comments mark the regions the book prints. They are how the book's build
finds its listings, and they have no effect on the code.

## The JDK's code

This repository contains no OpenJDK source or class files. The tests compile the book's own
Java programs with your JDK, and from Chapter 10 carafe reads the JDK's class library from
your installation at run time.

Java and OpenJDK are trademarks or registered trademarks of Oracle and/or its affiliates.

## Found a mistake?

Please open an issue, whether it is a bug in the code or a place where the book and the
repository disagree.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option. The license covers the code in this repository.
The text of the book is not part of it.
