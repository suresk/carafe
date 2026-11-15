//! A class as the running VM holds it.

use crate::bytecode::Instruction;
use crate::class_file::ClassFile;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::{HEADER, size_of};
use std::cell::OnceCell;
use std::rc::Rc;

// @listing class
/// A class the VM can run: its parsed class file, and each method's
/// instructions, decoded the first time the method is called.
pub struct Class {
    pub file: ClassFile,
    code: Vec<OnceCell<Rc<[Instruction]>>>,
    // @hide
    /// Where the instance fields are in an object of this class.
    pub layout: Layout,
    /// The superclass, or `None` for `Object` (and, until Chapter 10, for the
    /// classes that extend it).
    pub superclass: Option<Rc<Class>>,
    /// The direct superinterfaces.
    pub interfaces: Vec<Rc<Class>>,
    // @show
}

impl Class {
    pub fn new(file: ClassFile) -> Class {
        let code = file.methods.iter().map(|_| OnceCell::new()).collect();
        // @hide
        let layout = Layout::new(&file, &Layout::OBJECT);
        // @show
        Class {
            file,
            code,
            // @hide
            layout,
            superclass: None,
            interfaces: Vec::new(),
            // @show
        }
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

// @listing layout
/// Where the instance fields of a class are in its objects: superclass
/// fields first, then the class's own, widest first.
#[derive(Clone, Debug)]
pub struct Layout {
    /// The bytes an object takes, header included, before rounding up to 8.
    pub size: u32,
    pub fields: Vec<Slot>,
}

/// One instance field and its offset from the start of the object.
#[derive(Clone, Debug)]
pub struct Slot {
    pub name: String,
    pub descriptor: String,
    pub ty: FieldType,
    pub offset: u32,
}

impl Layout {
    /// The layout of `java.lang.Object`, which has no fields.
    pub const OBJECT: Layout = Layout { size: HEADER, fields: Vec::new() };

    /// The layout of `file`'s objects, whose superclass's objects are laid
    /// out as `superclass`.
    pub fn new(file: &ClassFile, superclass: &Layout) -> Layout {
        let mut own: Vec<_> = file.fields.iter().collect();
        own.retain(|f| !f.access_flags.has(AccessFlags::STATIC));
        own.sort_by_key(|f| std::cmp::Reverse(size_of(&f.ty))); // stable
        let mut fields = superclass.fields.clone();
        let mut end = superclass.size;
        for field in own {
            let size = size_of(&field.ty);
            let offset = end.next_multiple_of(size);
            let (name, descriptor) = (field.name.clone(), field.descriptor.clone());
            fields.push(Slot { name, descriptor, ty: field.ty.clone(), offset });
            end = offset + size;
        }
        Layout { size: end, fields }
    }
}
// @end

impl Class {
    // @listing extending
    /// A class whose superclass is `superclass`, with its fields laid out
    /// after the superclass's.
    pub fn extending(file: ClassFile, superclass: Rc<Class>) -> Class {
        let mut class = Class::new(file);
        class.layout = Layout::new(&class.file, &superclass.layout);
        class.superclass = Some(superclass);
        class
    }
    // @end
}

impl Class {
    // @listing class-field
    /// The instance field with this name and descriptor, searching the
    /// class's own fields before its superclasses'.
    pub fn field(&self, name: &str, descriptor: &str) -> Result<&Slot> {
        let fields = &self.layout.fields;
        let found = fields
            .iter()
            .rev()
            .find(|f| f.name == name && f.descriptor == descriptor);
        found.ok_or_else(|| Error::Exception {
            class: "java/lang/NoSuchFieldError",
            message: Some(name.to_string()),
        })
    }
    // @end
}
