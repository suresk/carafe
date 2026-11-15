//! The JDK that carafe's tests compile with.

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, PoisonError};
use std::{env, fs};

// @listing jdk
/// The `JAVA_RUNTIME_VERSION` of the one JDK carafe's numbers are true for:
/// Eclipse Temurin 25.0.5+8.
pub const JDK_VERSION: &str = "25.0.5+8-LTS";

/// A JDK home directory, checked to be the pinned release.
pub struct Jdk {
    home: PathBuf,
}
// @end

impl Jdk {
    // @listing jdk-find
    /// The JDK named by `CARAFE_JDK`, or failing that by `JAVA_HOME`.
    pub fn find() -> Result<Jdk> {
        let home = env::var_os("CARAFE_JDK").or_else(|| env::var_os("JAVA_HOME"));
        let Some(home) = home.map(PathBuf::from) else {
            let m = format!(
                "set CARAFE_JDK to the home directory of Temurin {JDK_VERSION}"
            );
            return Err(Error::Jdk(m));
        };
        let release = fs::read_to_string(home.join("release")).map_err(|e| {
            Error::Jdk(format!("{}: no release file ({e})", home.display()))
        })?;
        let field = |key: &str| {
            let value =
                release.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix('='));
            value.unwrap_or("").trim_matches('"').to_string()
        };
        let (vendor, version) = (field("IMPLEMENTOR"), field("JAVA_RUNTIME_VERSION"));
        if vendor != "Eclipse Adoptium" || version != JDK_VERSION {
            return Err(Error::Jdk(format!(
                "{} is {vendor} {version}, not Temurin {JDK_VERSION}",
                home.display()
            )));
        }
        Ok(Jdk { home })
    }
    // @end

    // @listing jdk-javac
    /// Compiles `source` with `javac` into the directory `out`, once per process.
    pub fn javac(&self, source: &Path, out: &Path) -> Result<()> {
        static DONE: Mutex<Vec<(PathBuf, PathBuf)>> = Mutex::new(Vec::new());
        let mut done = DONE.lock().unwrap_or_else(PoisonError::into_inner);
        let job = (source.to_path_buf(), out.to_path_buf());
        if !done.contains(&job) {
            let mut javac = Command::new(self.home.join("bin").join("javac"));
            self.run(javac.arg("-d").arg(out).arg(source))?;
            done.push(job);
        }
        Ok(())
    }

    /// The output of `javap -v -p` for the class file at `class`.
    pub fn javap(&self, class: &Path) -> Result<String> {
        let mut javap = Command::new(self.home.join("bin").join("javap"));
        self.run(javap.args(["-v", "-p"]).arg(class))
    }

    fn run(&self, command: &mut Command) -> Result<String> {
        let output = command.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Jdk(format!("{command:?} failed:\n{stderr}")));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
    // @end
}

impl Jdk {
    // @listing jdk-javap-code
    /// The output of `javap -c -p`: every method's bytecode, disassembled.
    pub fn javap_code(&self, class: &Path) -> Result<String> {
        let mut javap = Command::new(self.home.join("bin").join("javap"));
        self.run(javap.args(["-c", "-p"]).arg(class))
    }
    // @end
}

impl Jdk {
    // @listing jdk-java
    /// Runs the `main` method of `class` on the JDK's own JVM, HotSpot, with
    /// the directory `classes` as the class path, and returns what it prints.
    pub fn java(&self, classes: &Path, class: &str) -> Result<String> {
        let mut java = Command::new(self.home.join("bin").join("java"));
        self.run(java.arg("-cp").arg(classes).arg(class))
    }
    // @end
}

impl Jdk {
    // @listing jdk-java-with
    /// As `java`, with `options` for the JVM before the class name.
    pub fn java_with(
        &self,
        options: &[&str],
        classes: &Path,
        class: &str,
    ) -> Result<String> {
        let mut java = Command::new(self.home.join("bin").join("java"));
        self.run(java.args(options).arg("-cp").arg(classes).arg(class))
    }
    // @end
}
