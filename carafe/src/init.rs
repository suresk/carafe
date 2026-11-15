//! Static fields and class initialization (JVMS 5.5).

use crate::cache::{Resolved, initialized};
use crate::class::{Class, State, coerce};
use crate::error::{Error, Result};
use crate::flags::AccessFlags;
use crate::frame::Frame;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::opcode::GETSTATIC;
use crate::pool::Constant;
use crate::resolve::dotted;
use crate::value::Value;
use std::rc::Rc;

impl Vm {
    // @listing initialize
    /// Initializes `class` unless it is initialized or being initialized
    /// (JVMS 5.5). An initializer that throws leaves the class erroneous.
    pub fn initialize(&mut self, class: &Rc<Class>) -> Result<()> {
        match class.state.get() {
            State::Initialized | State::InProgress => return Ok(()),
            State::Erroneous => {
                let m = format!(
                    "Could not initialize class {}",
                    dotted(&class.file.this_class)
                );
                return Err(linkage("java/lang/NoClassDefFoundError", m));
            }
            State::Uninitialized => {}
        }
        // @hide
        if class.file.this_class == "jdk/internal/reflect/ReflectionFactory" {
            self.before_reflection()?;
        }
        // @show
        class.state.set(State::InProgress);
        let result = self.run_initializers(class);
        class.state.set(match result {
            Ok(()) => State::Initialized,
            Err(_) => State::Erroneous,
        });
        // @hide
        let result = result.map_err(|e| self.in_initializer(e));
        // @show
        result.map_err(|e| match e {
            // A Java exception that isn't an Error is wrapped (JVMS 5.5,
            // step 11). Until Chapter 16, an Error is any class whose name
            // ends in "Error", as every JVM-thrown Error's does.
            Error::Exception { class, .. } if !class.ends_with("Error") => {
                Error::Exception {
                    class: "java/lang/ExceptionInInitializerError",
                    message: None,
                }
            }
            other => other,
        })
    }
    // @end

    // @listing run-initializers
    /// Steps 6 to 9 of JVMS 5.5: constant values, then the superclass and the
    /// interfaces that declare default methods, then the class's `<clinit>`.
    fn run_initializers(&mut self, class: &Rc<Class>) -> Result<()> {
        let static_fields = class.file.fields.iter();
        let static_fields =
            static_fields.filter(|f| f.access_flags.has(AccessFlags::STATIC));
        for (slot, field) in static_fields.enumerate() {
            if let Some(index) = field.constant_value {
                let value = match class.file.constant_pool.get(index)? {
                    Constant::Integer(v) => Value::Int(*v),
                    Constant::Long(v) => Value::Long(*v),
                    Constant::Float(v) => Value::Float(*v),
                    Constant::Double(v) => Value::Double(*v),
                    // @hide
                    Constant::String { .. } => {
                        Value::Ref(self.string_constant(class, index)?)
                    }
                    // @show
                    other => {
                        return Err(Error::Unsupported(format!(
                            "a constant {other:?}"
                        )));
                    }
                };
                class.statics.borrow_mut()[slot] = coerce(&field.ty, value)?;
            }
        }
        if !class.file.access_flags.has(AccessFlags::INTERFACE) {
            if let Some(superclass) = &class.superclass {
                self.initialize(superclass)?;
            }
            for interface in &class.interfaces {
                let defaults = interface.file.methods.iter().any(|m| {
                    !m.access_flags.has(AccessFlags::ABSTRACT | AccessFlags::STATIC)
                });
                if defaults {
                    self.initialize(interface)?;
                }
            }
        }
        let Ok(clinit) = class.method("<clinit>", "()V") else { return Ok(()) };
        let depth = self.frames.len();
        self.call(Frame::new(class.clone(), clinit, &[])?)?;
        let result = self.run(depth);
        self.frames.truncate(depth);
        result.map(|_| ())
    }
    // @end

    // @listing static-op
    /// `getstatic` and `putstatic`, which initialize the class that declares
    /// the field before using it.
    pub(crate) fn static_op(&mut self, opcode: u8, index: u16) -> Result<()> {
        let from = self.frames.last().expect("a frame").class.clone();
        // @hide
        if let Some(Resolved::Static(owner, slot, ty)) = from.cached(index) {
            return self.static_access(opcode, owner, *slot, ty);
        }
        // @show
        let (owner, field) = self.resolve_field(&from, index)?;
        if !field.access_flags.has(AccessFlags::STATIC) {
            let m = format!(
                "Expected static field {}.{}",
                dotted(&owner.file.this_class),
                field.name
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        self.initialize(&owner)?;
        let slot =
            owner.static_slot(&field.name, &field.descriptor).expect("declared");
        // @hide
        if initialized(&owner) {
            from.cache(
                index,
                Resolved::Static(owner.clone(), slot, field.ty.clone()),
            );
        }
        // @show
        let frame = self.frames.last_mut().expect("a frame");
        if opcode == GETSTATIC {
            let value = owner.statics.borrow()[slot];
            frame.push(value)
        } else {
            let value = coerce(&field.ty, frame.pop()?)?;
            owner.statics.borrow_mut()[slot] = value;
            Ok(())
        }
    }
    // @end
}
