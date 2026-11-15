//! A class as the running VM holds it.

use crate::bytecode::Instruction;
use crate::class_file::ClassFile;
use crate::error::{Error, Result};
use std::cell::OnceCell;
use std::rc::Rc;

// @listing class
/// A class the VM can run: its parsed class file, and each method's
/// instructions, decoded the first time the method is called.
pub struct Class {
    pub file: ClassFile,
    code: Vec<OnceCell<Rc<[Instruction]>>>,
}

impl Class {
    pub fn new(file: ClassFile) -> Class {
        let code = file.methods.iter().map(|_| OnceCell::new()).collect();
        Class { file, code }
    }

    /// The index of the method with this name and descriptor.
    pub fn method(&self, name: &str, descriptor: &str) -> Result<usize> {
        let methods = &self.file.methods;
        let found =
            methods.iter().position(|m| m.name == name && m.descriptor == descriptor);
        found.ok_or_else(|| Error::Exception {
            class: "java/lang/NoSuchMethodError",
            message: Some(format!("{}.{name}{descriptor}", self.file.this_class)),
        })
    }

    /// The instructions of method `index`, decoded on the first call.
    pub fn instructions(&self, index: usize) -> Result<Rc<[Instruction]>> {
        if let Some(decoded) = self.code[index].get() {
            return Ok(decoded.clone());
        }
        let method = &self.file.methods[index];
        let Some(code) = &method.code else {
            let m =
                format!("{}{}, which has no code", method.name, method.descriptor);
            return Err(Error::Unsupported(m));
        };
        let decoded: Rc<[Instruction]> = code.decode()?.into();
        Ok(self.code[index].get_or_init(|| decoded).clone())
    }
}
// @end
