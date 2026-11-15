//! Native methods: the parts of the class library written in Rust.

use crate::error::Result;
use crate::frame::Frame;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::resolve::external;
use crate::value::Value;

// @listing native-fn
/// A native method: the VM and the arguments, `this` first for an instance
/// method; it returns the method's result.
pub type Native = fn(&mut Vm, &[Value]) -> Result<Option<Value>>;

/// The native method `class.name` with this descriptor, if carafe has it.
fn find(class: &str, name: &str, descriptor: &str) -> Option<Native> {
    Some(match (class, name, descriptor) {
        ("java/io/PrintStream", "write0", "(ILjava/lang/String;)V") => write0,
        _ => return None,
    })
}
// @end

impl Vm {
    // @listing call-native
    /// Runs a native method in place of a frame of bytecode, and pushes its
    /// result onto the caller's operand stack.
    pub(crate) fn call_native(&mut self, frame: Frame) -> Result<()> {
        let class = &frame.class.file.this_class;
        let method = &frame.class.file.methods[frame.method];
        let Some(native) = find(class, &method.name, &method.descriptor) else {
            let m = format!("'{}'", external(class, &method.name, &method.ty));
            return Err(linkage("java/lang/UnsatisfiedLinkError", m));
        };
        self.natives.insert(format!("{class}.{}{}", method.name, method.descriptor));
        if let Some(result) = native(self, &frame.args())? {
            self.frames.last_mut().expect("a caller").push(result)?;
        }
        Ok(())
    }
    // @end
}

// @listing write0
/// `PrintStream.write0(int fd, String s)`, in carafe's `PrintStream`: writes
/// the string, as UTF-8, to standard output or standard error.
fn write0(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Int(fd), Value::Ref(s)] = *args else {
        unreachable!("checked descriptor")
    };
    let text = vm.rust_string(s)?;
    match fd {
        2 => vm.stderr.extend_from_slice(text.as_bytes()),
        _ => vm.stdout.extend_from_slice(text.as_bytes()),
    }
    Ok(None)
}
// @end
