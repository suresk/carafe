//! `SwitchBootstraps.typeSwitch`, and the sealed classes it often switches on.

use crate::class::Class;
use crate::descriptor::FieldType;
use crate::dynamic::{Bootstrap, CallSite, bootstrap_methods};
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::pool::{Constant, ConstantPool};
use crate::reader::Reader;
use crate::resolve::dotted;
use crate::value::Value;
use std::rc::Rc;

// @listing label
/// A `case` label of a pattern `switch`: a type, which matches any instance
/// of it; a constant, which matches an equal boxed `int` or string; an enum
/// constant, by its class and name; or a primitive type, which matches a
/// selector of that type.
pub enum Label {
    Type(Rc<Class>),
    Int(i32),
    String(Vec<u16>),
    Enum { class: Rc<Class>, name: String },
    Primitive(FieldType),
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
                Constant::Dynamic { .. } => self.dynamic_label(caller, index)?,
                ref other => {
                    let m = format!("a switch label {other:?}");
                    return Err(Error::Unsupported(m));
                }
            });
        }
        Ok(CallSite::TypeSwitch { labels })
    }

    /// A label that is a `Dynamic` constant, recognized by its bootstrap
    /// method without running it: `ConstantBootstraps.primitiveClass` names
    /// a primitive type, and `ConstantBootstraps.invoke` of `EnumDesc.of`,
    /// with a `ClassDesc.of` the same way, an enum constant.
    fn dynamic_label(&mut self, caller: &Class, index: u16) -> Result<Label> {
        let pool = &caller.file.constant_pool;
        let bootstraps = bootstrap_methods(&caller.file)?;
        let bootstrap = |index| dynamic(pool, &bootstraps, index);
        let is = |index, method| handle(pool, index).is_ok_and(|h| h == method);
        let unsupported = || {
            let constant = pool.get(index).map(|c| format!("{c:?}"));
            Error::Unsupported(format!(
                "a switch label {}",
                constant.unwrap_or_default()
            ))
        };
        let constants = "java/lang/invoke/ConstantBootstraps";
        let invoke = (constants, "invoke");
        match bootstrap(index)? {
            (name, (c, "primitiveClass"), _) if c == constants => {
                match FieldType::parse(name)? {
                    FieldType::Object(_) | FieldType::Array(_) => Err(unsupported()),
                    primitive => Ok(Label::Primitive(primitive)),
                }
            }
            (_, m, &[of, desc, name])
                if m == invoke && is(of, ("java/lang/Enum$EnumDesc", "of")) =>
            {
                let (_, m, &[class_of, class]) = bootstrap(desc)? else {
                    return Err(unsupported());
                };
                if m != invoke
                    || !is(class_of, ("java/lang/constant/ClassDesc", "of"))
                {
                    return Err(unsupported());
                }
                let class = string(pool, class)?.replace('.', "/");
                let class = self.resolve_class(caller, &class)?;
                Ok(Label::Enum { class, name: string(pool, name)?.to_string() })
            }
            _ => Err(unsupported()),
        }
    }
    // @end

    // @listing type-switch
    /// Runs a type switch: pops the index to restart from and the selector,
    /// an object or a primitive, and pushes the index of the first label from
    /// there that matches it, -1 for `null`, or the number of labels if none
    /// matches, as for a restart index past the end.
    pub(crate) fn type_switch(&mut self, labels: &[Label]) -> Result<()> {
        let frame = self.frames.last_mut().expect("a frame");
        let restart = frame.pop_int()?;
        let selector = frame.pop()?;
        let mut index = -1;
        if selector != Value::Ref(Ref::NULL) {
            let start = usize::try_from(restart).unwrap_or(0);
            index = labels.len() as i32;
            for (i, label) in labels.iter().enumerate().skip(start) {
                if self.matches(label, selector)? {
                    index = i as i32;
                    break;
                }
            }
        }
        self.frames.last_mut().expect("a frame").push(Value::Int(index))
    }

    /// Whether `label` matches `selector`, which isn't `null`. `javac` gives
    /// a primitive selector only primitive labels of its own type.
    fn matches(&mut self, label: &Label, selector: Value) -> Result<bool> {
        let Value::Ref(object) = selector else {
            return Ok(matches!(label, Label::Primitive(t) if holds(selector, t)));
        };
        let class = self.class_of(object)?;
        Ok(match label {
            Label::Type(t) => self.is_assignable(&class, t)?,
            Label::Int(v) => self.boxed_int(object)? == Some(*v),
            Label::String(units) => {
                class.file.this_class == "java/lang/String"
                    && self.string_units(object)? == *units
            }
            Label::Enum { class: e, name } => {
                self.is_assignable(&class, e)? && self.enum_name(object)? == *name
            }
            Label::Primitive(_) => false,
        })
    }

    /// The name of an enum constant, from `Enum`'s `name` field.
    fn enum_name(&mut self, constant: Ref) -> Result<String> {
        let enum_class = self.load("java/lang/Enum")?;
        let slot = enum_class.field("name", "Ljava/lang/String;")?;
        match self.heap.read(constant.0 + slot.offset, &slot.ty) {
            Value::Ref(name) if name != Ref::NULL => self.rust_string(name),
            _ => Ok(String::new()),
        }
    }

    /// The value in a `Character`, `Byte`, `Short` or `Integer`, which an
    /// `int` label matches. (JDK 25's `SwitchBootstraps` also matches other
    /// `Number`s by `intValue`, which `javac` output never needs.)
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

// @listing label-constants
/// Whether `value` is a value of the primitive type `ty`.
fn holds(value: Value, ty: &FieldType) -> bool {
    use FieldType::*;
    matches!(
        (value, ty),
        (Value::Int(_), Boolean | Byte | Char | Short | Int)
            | (Value::Long(_), Long)
            | (Value::Float(_), Float)
            | (Value::Double(_), Double)
    )
}

/// The `Dynamic` constant `index`: its name, its bootstrap method as a class
/// and a method name, and the static arguments passed to it.
fn dynamic<'p>(
    pool: &'p ConstantPool,
    bootstraps: &'p [Bootstrap],
    index: u16,
) -> Result<(&'p str, (&'p str, &'p str), &'p [u16])> {
    let &Constant::Dynamic { bootstrap, name_and_type } = pool.get(index)? else {
        return Err(Error::ClassFormat(format!("#{index} is not a Dynamic")));
    };
    let Some(b) = bootstraps.get(usize::from(bootstrap)) else {
        return Err(Error::ClassFormat(format!("no bootstrap method #{bootstrap}")));
    };
    let (name, _) = pool.name_and_type(name_and_type)?;
    Ok((name, handle(pool, b.method)?, &b.args))
}

/// The class and name of the method of the `MethodHandle` constant `index`.
fn handle(pool: &ConstantPool, index: u16) -> Result<(&str, &str)> {
    let &Constant::MethodHandle { reference, .. } = pool.get(index)? else {
        return Err(Error::ClassFormat(format!("#{index} is not a MethodHandle")));
    };
    let method = pool.member(reference)?;
    Ok((method.class, method.name))
}

/// The text of the `String` constant `index`.
fn string(pool: &ConstantPool, index: u16) -> Result<&str> {
    let &Constant::String { utf8 } = pool.get(index)? else {
        return Err(Error::ClassFormat(format!("#{index} is not a String")));
    };
    pool.utf8(utf8)
}
// @end

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
