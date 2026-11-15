//! `SwitchBootstraps.typeSwitch`, and the sealed classes it often switches on.

use crate::class::Class;
use crate::dynamic::CallSite;
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::pool::Constant;
use crate::reader::Reader;
use crate::resolve::dotted;
use crate::value::Value;
use std::rc::Rc;

// @listing label
/// A `case` label of a pattern `switch`: a type, which matches any instance
/// of it, or a constant, which matches an equal boxed `int` or string.
pub enum Label {
    Type(Rc<Class>),
    Int(i32),
    String(Vec<u16>),
}
// @end

impl Vm {
    // @listing type-switch-site
    /// Links a call site of `SwitchBootstraps.typeSwitch`: one label for each
    /// static argument, in `case` order, with the classes resolved now.
    pub(crate) fn type_switch_site(
        &mut self,
        caller: &Class,
        statics: &[u16],
    ) -> Result<CallSite> {
        let pool = &caller.file.constant_pool;
        let mut labels = Vec::new();
        for &index in statics {
            labels.push(match *pool.get(index)? {
                Constant::Class { name } => {
                    Label::Type(self.resolve_class(caller, pool.utf8(name)?)?)
                }
                Constant::Integer(v) => Label::Int(v),
                Constant::String { .. } => {
                    let r = self.string_constant(caller, index)?;
                    Label::String(self.string_units(r)?)
                }
                ref other => {
                    let m = format!("a switch label {other:?}");
                    return Err(Error::Unsupported(m));
                }
            });
        }
        Ok(CallSite::TypeSwitch { labels })
    }
    // @end

    // @listing type-switch
    /// Runs a type switch: pops the index to restart from and the object,
    /// and pushes the index of the first label from there that matches it,
    /// -1 for `null`, or the number of labels if none matches.
    pub(crate) fn type_switch(&mut self, labels: &[Label]) -> Result<()> {
        let frame = self.frames.last_mut().expect("a frame");
        let restart = frame.pop_int()?;
        let Value::Ref(target) = frame.pop()? else {
            return Err(Error::Verify("typeSwitch of a primitive".into()));
        };
        let mut index = -1;
        if target != Ref::NULL {
            let class = self.class_of(target)?;
            let start = usize::try_from(restart).unwrap_or(0);
            index = labels.len() as i32;
            for (i, label) in labels.iter().enumerate().skip(start) {
                let matches = match label {
                    Label::Type(t) => self.is_assignable(&class, t)?,
                    Label::Int(v) => self.boxed_int(target)? == Some(*v),
                    Label::String(units) => {
                        class.file.this_class == "java/lang/String"
                            && self.string_units(target)? == *units
                    }
                };
                if matches {
                    index = i as i32;
                    break;
                }
            }
        }
        self.frames.last_mut().expect("a frame").push(Value::Int(index))
    }

    /// The value in a `Character`, `Byte`, `Short` or `Integer`, which an
    /// `int` label matches.
    fn boxed_int(&mut self, object: Ref) -> Result<Option<i32>> {
        let class = self.class_of(object)?;
        let ty = match class.file.this_class.as_str() {
            "java/lang/Integer" => "I",
            "java/lang/Character" => "C",
            "java/lang/Short" => "S",
            "java/lang/Byte" => "B",
            _ => return Ok(None),
        };
        let slot = class.field("value", ty)?;
        match self.heap.read(object.0 + slot.offset, &slot.ty) {
            Value::Int(v) => Ok(Some(v)),
            _ => Ok(None),
        }
    }
    // @end
}

// @listing permitted
/// JVMS 5.3.5: a class whose superclass or a superinterface is sealed must
/// be among the classes that its `PermittedSubclasses` attribute lists.
pub(crate) fn check_permitted(class: &Class) -> Result<()> {
    let supers = class.superclass.iter().chain(&class.interfaces);
    for sealed in supers {
        let attributes = &sealed.file.attributes;
        let Some(permitted) =
            attributes.iter().find(|a| a.name == "PermittedSubclasses")
        else {
            continue;
        };
        let pool = &sealed.file.constant_pool;
        let mut r = Reader::new(&permitted.info);
        let mut listed = false;
        for _ in 0..r.u2()? {
            listed |= pool.class_name(r.u2()?)? == class.file.this_class;
        }
        if !listed {
            let m = format!(
                "Failed listed permitted subclass check: class {} is not a permitted \
                 subclass of {}",
                dotted(&class.file.this_class),
                dotted(&sealed.file.this_class)
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
    }
    Ok(())
}
// @end
