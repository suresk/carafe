//! A class as the running VM holds it.

use crate::bytecode::Instruction;
use crate::cache::Resolved;
use crate::class_file::ClassFile;
use crate::descriptor::FieldType;
use crate::dynamic::CallSite;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::{HEADER, Ref, size_of};
use crate::value::Value;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
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
    /// How far initialization has got (JVMS 5.5).
    pub state: Cell<State>,
    /// The values of the static fields, in the order the class declares them.
    pub statics: RefCell<Vec<Value>>,
    /// The class's `java.lang.Class` object, once something has asked for it.
    pub mirror: Cell<Ref>,
    /// Whether the class came from the boot class path: `java.base` and the
    /// shims, as opposed to the program's own classes.
    pub boot: bool,
    /// The linked `invokedynamic` call sites, by method index and pc.
    pub call_sites: RefCell<HashMap<(usize, u16), Rc<CallSite>>>,
    /// What each constant pool entry resolved to, by index, once it has been.
    pub(crate) resolved: Box<[OnceCell<Resolved>]>,
    // @show
}

impl Class {
    pub fn new(file: ClassFile) -> Class {
        let code = file.methods.iter().map(|_| OnceCell::new()).collect();
        // @hide
        let layout = Layout::new(&file, &Layout::OBJECT);
        let static_fields =
            file.fields.iter().filter(|f| f.access_flags.has(AccessFlags::STATIC));
        let statics = static_fields.map(|f| zero(&f.ty)).collect();
        let count = file.constant_pool.count();
        // @show
        Class {
            file,
            code,
            // @hide
            layout,
            superclass: None,
            interfaces: Vec::new(),
            state: Cell::new(State::Uninitialized),
            statics: RefCell::new(statics),
            mirror: Cell::new(Ref::NULL),
            boot: false,
            call_sites: RefCell::new(HashMap::new()),
            resolved: (0..count).map(|_| OnceCell::new()).collect(),
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

// @listing state
/// The initialization state of a class, a single-threaded version of the
/// states of JVMS 5.5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Uninitialized,
    /// The initializer is running; requests to initialize return at once.
    InProgress,
    Initialized,
    /// The initializer threw; the class can never be used.
    Erroneous,
}
// @end

// @listing zero
/// The value a field of type `ty` has before anything stores to it.
pub fn zero(ty: &FieldType) -> Value {
    match ty {
        FieldType::Long => Value::Long(0),
        FieldType::Float => Value::Float(0.0),
        FieldType::Double => Value::Double(0.0),
        FieldType::Object(_) | FieldType::Array(_) => Value::Ref(Ref::NULL),
        _ => Value::Int(0), // int, and the narrow types
    }
}

/// `value`, converted for storing in a variable of type `ty`: narrowed if
/// `ty` is narrower than `int`, and refused if it is the wrong kind.
pub fn coerce(ty: &FieldType, value: Value) -> Result<Value> {
    Ok(match (ty, value) {
        (FieldType::Boolean, Value::Int(v)) => Value::Int(v & 1),
        (FieldType::Byte, Value::Int(v)) => Value::Int(v as i8 as i32),
        (FieldType::Char, Value::Int(v)) => Value::Int(v as u16 as i32),
        (FieldType::Short, Value::Int(v)) => Value::Int(v as i16 as i32),
        (FieldType::Int, Value::Int(_))
        | (FieldType::Long, Value::Long(_))
        | (FieldType::Float, Value::Float(_))
        | (FieldType::Double, Value::Double(_))
        | (FieldType::Object(_) | FieldType::Array(_), Value::Ref(_)) => value,
        _ => return Err(Error::Verify(format!("storing {value:?} as {ty}"))),
    })
}
// @end

impl Class {
    // @listing static-slot
    /// The index in `statics` of the static field with this name and
    /// descriptor, which the class must declare.
    pub fn static_slot(&self, name: &str, descriptor: &str) -> Option<usize> {
        let fields = self.file.fields.iter();
        let mut statics = fields.filter(|f| f.access_flags.has(AccessFlags::STATIC));
        statics.position(|f| f.name == name && f.descriptor == descriptor)
    }
    // @end
}
