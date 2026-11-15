//! Loading classes from a class path (JVMS 5.3).

use crate::class::Class;
use crate::class_file::ClassFile;
use crate::descriptor::FieldType;
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::pool::ConstantPool;
use crate::reader::Reader;
use std::path::PathBuf;
use std::rc::Rc;

// @listing class-path
/// Where classes come from: directories of class files, searched in order.
/// The class `a/b/C` is the file `a/b/C.class` under one of them.
#[derive(Clone, Debug, Default)]
pub struct ClassPath {
    pub dirs: Vec<PathBuf>,
}

impl ClassPath {
    /// The bytes of the class file for `name`, from the first directory
    /// that has one.
    pub fn find(&self, name: &str) -> Result<Option<Vec<u8>>> {
        for dir in &self.dirs {
            let path = dir.join(format!("{name}.class"));
            if path.is_file() {
                return Ok(Some(std::fs::read(path)?));
            }
        }
        Ok(None)
    }
}
// @end

// @listing linkage-error
/// One of the `LinkageError`s that loading and resolution throw.
pub(crate) fn linkage(class: &'static str, message: String) -> Error {
    Error::Exception { class, message: Some(message) }
}
// @end

impl Vm {
    // @listing load
    /// The class called `name`, loaded from the class path the first time it
    /// is asked for, with its superclass and interfaces.
    pub fn load(&mut self, name: &str) -> Result<Rc<Class>> {
        if let Some(class) = self.classes.get(name) {
            return Ok(class.clone());
        }
        // @hide
        if name.starts_with('[') {
            return self.load_array_class(name);
        }
        // @show
        let Some(bytes) = self.class_path.find(name)? else {
            return Err(linkage("java/lang/NoClassDefFoundError", name.to_string()));
        };
        let file = ClassFile::parse(&bytes)?;
        if file.this_class != name {
            let m = format!("{name} (wrong name: {})", file.this_class);
            return Err(linkage("java/lang/NoClassDefFoundError", m));
        }
        self.loading.push(name.to_string());
        let defined = self.define(file);
        self.loading.pop();
        // @hide
        let defined = defined.map(|mut class| {
            let boot = self.class_path.dirs.iter().take(self.boot_dirs);
            class.boot = boot
                .into_iter()
                .any(|dir| dir.join(format!("{name}.class")).is_file());
            class
        });
        // @show
        let class = Rc::new(defined?);
        self.classes.insert(name.to_string(), class.clone());
        self.loaded.push(name.to_string());
        Ok(class)
    }
    // @end

    // @listing define
    /// Makes a `Class` of a parsed class file, after loading its direct
    /// superinterfaces and then its superclass, as HotSpot does.
    fn define(&mut self, file: ClassFile) -> Result<Class> {
        let name = file.this_class.clone();
        let mut interfaces = Vec::new();
        for interface in &file.interfaces {
            let loaded = self.load_super(&name, interface)?;
            if !loaded.file.access_flags.has(AccessFlags::INTERFACE) {
                let m = format!(
                    "class {name} cannot implement {interface}, \
                                 because it is not an interface"
                );
                return Err(linkage("java/lang/IncompatibleClassChangeError", m));
            }
            interfaces.push(loaded);
        }
        let mut class = match &file.super_class {
            // Object is a stand-in until Chapter 10 puts java.base on the path.
            Some(s)
                if s == "java/lang/Object" && self.class_path.find(s)?.is_none() =>
            {
                Class::new(file)
            }
            Some(superclass) => {
                let superclass = self.load_super(&name, superclass)?;
                let flags = superclass.file.access_flags;
                if flags.has(AccessFlags::INTERFACE) {
                    let m = format!(
                        "class {name} has interface {} as super class",
                        superclass.file.this_class
                    );
                    return Err(linkage("java/lang/IncompatibleClassChangeError", m));
                }
                if flags.has(AccessFlags::FINAL) {
                    return Err(Error::Verify(
                        "Cannot inherit from final class".into(),
                    ));
                }
                Class::extending(file, superclass)
            }
            None => Class::new(file),
        };
        class.interfaces = interfaces;
        // @hide
        crate::switches::check_permitted(&class)?;
        // @show
        Ok(class)
    }

    /// Loads a superclass or superinterface of `name`, which must not be a
    /// class that is itself still being loaded.
    fn load_super(&mut self, name: &str, superclass: &str) -> Result<Rc<Class>> {
        if self.loading.iter().any(|n| n == superclass) {
            return Err(linkage("java/lang/ClassCircularityError", name.to_string()));
        }
        self.load(superclass)
    }
    // @end
}

impl Vm {
    // @listing load-array-class
    /// An array class, created by the VM rather than read from a class file
    /// (JVMS 5.3.3): `public final abstract`, a subclass of `Object` that
    /// implements `Cloneable` and `Serializable`, with no fields or methods
    /// of its own. A reference element type's class is loaded first.
    fn load_array_class(&mut self, name: &str) -> Result<Rc<Class>> {
        let FieldType::Array(element) = FieldType::parse(name)? else {
            return Err(linkage("java/lang/NoClassDefFoundError", name.to_string()));
        };
        match *element {
            FieldType::Object(ref class) => {
                self.load(class)?;
            }
            FieldType::Array(_) => {
                self.load(&element.descriptor())?;
            }
            _ => {}
        }
        let object = self.load("java/lang/Object")?;
        let interfaces = ["java/lang/Cloneable", "java/io/Serializable"];
        let interfaces: Vec<Rc<Class>> =
            interfaces.iter().map(|i| self.load(i)).collect::<Result<_>>()?;
        let flags = AccessFlags::PUBLIC | AccessFlags::FINAL | AccessFlags::ABSTRACT;
        let file = ClassFile {
            minor_version: 0,
            major_version: 69,
            constant_pool: ConstantPool::read(&mut Reader::new(&[0, 1]))?, // empty
            access_flags: AccessFlags(flags),
            this_class: name.to_string(),
            super_class: Some("java/lang/Object".to_string()),
            interfaces: interfaces
                .iter()
                .map(|i| i.file.this_class.clone())
                .collect(),
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
        };
        let mut class = Class::extending(file, object);
        class.interfaces = interfaces;
        // @hide
        class.boot = match *element {
            FieldType::Object(ref name) => self.load(name)?.boot,
            FieldType::Array(_) => self.load(&element.descriptor())?.boot,
            _ => true,
        };
        // @show
        let class = Rc::new(class);
        self.classes.insert(name.to_string(), class.clone());
        self.loaded.push(name.to_string());
        Ok(class)
    }

    /// The class of the object or array at `r`.
    pub fn class_of(&mut self, r: Ref) -> Result<Rc<Class>> {
        if self.heap.is_array(r) {
            let element = self.heap.element_type(r).clone();
            return self.load(&FieldType::Array(Box::new(element)).descriptor());
        }
        Ok(self.heap.class_of(r).clone())
    }
    // @end
}
