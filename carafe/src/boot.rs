//! Starting a VM on the JDK's class library, and running a program.

use crate::descriptor::FieldType;
use crate::error::Result;
use crate::interpreter::Vm;
use crate::jdk::Jdk;
use crate::loader::ClassPath;
use crate::value::Value;
use std::path::Path;

// @listing shim-dir
/// Where carafe's shims, `java/lang/System.java` and the rest, are.
pub const SHIMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shim");
// @end

impl Vm {
    // @listing boot
    /// A VM whose class path is carafe's shims, then the JDK's `java.base`,
    /// then `app`. `work` is a directory for the extracted and compiled
    /// classes, reused between runs.
    pub fn boot(jdk: &Jdk, work: &Path, app: &Path) -> Result<Vm> {
        std::fs::create_dir_all(work)?;
        let shims = jdk.shims(Path::new(SHIMS), work)?;
        let java_base = jdk.java_base(work)?;
        let mut vm = Vm::new();
        vm.class_path = ClassPath { dirs: vec![shims, java_base, app.to_path_buf()] };
        // @hide
        vm.boot_dirs = 2;
        // @show
        Ok(vm)
    }

    /// Runs `class`'s `main` method with no arguments, as `java class` does.
    pub fn run_main(&mut self, class: &str) -> Result<()> {
        let class = self.load(class)?;
        let string = FieldType::Object("java/lang/String".to_string());
        let args = self.heap.new_array(&string, 0)?;
        self.invoke(&class, "main", "([Ljava/lang/String;)V", &[Value::Ref(args)])?;
        Ok(())
    }
    // @end
}
