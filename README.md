# carafe

carafe is a Java virtual machine written in plain Rust. It reads the class files `javac`
writes and runs them on the real JDK class library: `String`, `StringBuilder`, `ArrayList`,
`HashMap`, records, lambdas and exceptions. It has no dependencies.

It is the code for the book *Build a JVM in Rust* by Spencer Uresk, **which is still being
written**. The book builds carafe one chapter at a time, and every listing in it is printed
from this repository, so the code here is the code on the page.

carafe is for learning. It is not a replacement for HotSpot, and it is not a compatible
implementation of the Java SE specification.

## Where it stands

The repository grows with the book. So far it has the code for Chapter 1:

| Chapter | | Main files |
|---|---|---|
| 1 | A class file is a stream of bytes | `src/reader.rs`, `src/utf8.rs`, `src/pool.rs`, `src/class_file.rs`, `src/jdk.rs` |

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
$ cargo test                        # every chapter's examples and challenges
$ cargo xtask pool Hello.class      # a class file's version and constant pool
$ cargo xtask check 1               # Chapter 1's examples, and each of its challenges
$ cargo xtask check 1.3             # one challenge, with the full output
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
