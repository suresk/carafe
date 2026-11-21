//! `invokedynamic`: bootstrap methods, call sites and string concatenation.

use crate::class::Class;
use crate::class::Slot;
use crate::class_file::ClassFile;
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::error::{Error, Result};
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::pool::{Constant, ConstantPool};
use crate::reader::Reader;
use crate::records::RecordMethod;
use crate::switches::Label;
use crate::value::Value;
use std::cell::Cell;
use std::rc::Rc;

// @listing bootstrap
/// An entry of the `BootstrapMethods` attribute (JVMS 4.7.23): a
/// `MethodHandle` constant for the bootstrap method, and the constants
/// passed to it as its static arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bootstrap {
    pub method: u16,
    pub args: Vec<u16>,
}

/// The class's bootstrap methods, in the order `InvokeDynamic` entries
/// number them.
pub fn bootstrap_methods(file: &ClassFile) -> Result<Vec<Bootstrap>> {
    let Some(attribute) =
        file.attributes.iter().find(|a| a.name == "BootstrapMethods")
    else {
        return Ok(Vec::new());
    };
    let mut r = Reader::new(&attribute.info);
    let mut methods = Vec::new();
    for _ in 0..r.u2()? {
        let method = r.u2()?;
        let count = r.u2()?;
        let args = (0..count).map(|_| r.u2()).collect::<Result<Vec<u16>>>()?;
        methods.push(Bootstrap { method, args });
    }
    Ok(methods)
}
// @end

// @listing call-site
/// What an `invokedynamic` instruction does, decided once, when it is
/// first executed, by its bootstrap method.
pub enum CallSite {
    /// String concatenation: the pieces of the result, in order, and the
    /// types of the arguments the instruction pops.
    Concat { pieces: Vec<Piece>, args: Vec<FieldType> },
    // @hide
    /// A lambda: the class carafe made for it, the types of the values it
    /// captures, and its one object if it captures none.
    Lambda { class: Rc<Class>, captures: Vec<FieldType>, instance: Cell<Ref> },
    /// A record's `toString`, `equals` or `hashCode`: the record class and
    /// the fields of its components, in order.
    Record { method: RecordMethod, class: Rc<Class>, components: Vec<Slot> },
    /// A pattern `switch`: its labels, in `case` order.
    TypeSwitch { labels: Vec<Label> },
    // @show
}

/// A piece of a concatenation: text from the recipe, or argument `n`.
#[derive(Debug, PartialEq, Eq)]
pub enum Piece {
    Text(Vec<u16>),
    Arg(usize),
}
// @end

impl Vm {
    // @listing invoke-dynamic
    /// `invokedynamic`: links the call site the first time the instruction
    /// runs, and then executes the call site.
    pub(crate) fn invoke_dynamic(&mut self, index: u16, pc: u16) -> Result<()> {
        let frame = self.frames.last().expect("a frame");
        let (class, key) = (frame.class.clone(), (frame.method, pc));
        let site = class.call_sites.borrow().get(&key).cloned();
        let site = match site {
            Some(site) => site,
            None => {
                let site = Rc::new(self.link(&class, index)?);
                class.call_sites.borrow_mut().insert(key, site.clone());
                site
            }
        };
        match &*site {
            // @hide
            CallSite::Lambda { class, captures, instance } => {
                self.new_lambda(class, captures, instance)
            }
            CallSite::Record { method, class, components } => {
                self.record_method(*method, class, components)
            }
            CallSite::TypeSwitch { labels } => self.type_switch(labels),
            // @show
            CallSite::Concat { pieces, args } => self.concat(pieces, args),
        }
    }
    // @end
}

impl Vm {
    // @listing link
    /// Links the call site of the `InvokeDynamic` constant `index` by running
    /// its bootstrap method, which carafe knows by name.
    fn link(&mut self, class: &Class, index: u16) -> Result<CallSite> {
        let pool = &class.file.constant_pool;
        let &Constant::InvokeDynamic { bootstrap, name_and_type } =
            pool.get(index)?
        else {
            return Err(Error::Verify(format!("#{index} is not an InvokeDynamic")));
        };
        let (_, descriptor) = pool.name_and_type(name_and_type)?;
        let descriptor = MethodDescriptor::parse(descriptor)?;
        let bootstraps = bootstrap_methods(&class.file)?;
        let Some(b) = bootstraps.get(usize::from(bootstrap)) else {
            let m = format!("no bootstrap method #{bootstrap}");
            return Err(Error::ClassFormat(m));
        };
        let &Constant::MethodHandle { reference, .. } = pool.get(b.method)? else {
            return Err(Error::ClassFormat(format!(
                "#{} is not a MethodHandle",
                b.method
            )));
        };
        let method = pool.member(reference)?;
        match (method.class, method.name) {
            ("java/lang/invoke/StringConcatFactory", "makeConcatWithConstants") => {
                concat_site(pool, &b.args, descriptor)
                    .map_err(|error| self.bootstrap_failure(error))
            }
            // @hide
            (
                "java/lang/invoke/LambdaMetafactory",
                "metafactory" | "altMetafactory",
            ) => {
                let (name, _) = pool.name_and_type(name_and_type)?;
                self.lambda_site(class, name, descriptor, &b.args)
            }
            ("java/lang/runtime/ObjectMethods", "bootstrap") => {
                let (name, _) = pool.name_and_type(name_and_type)?;
                self.record_site(class, name, &b.args)
            }
            ("java/lang/runtime/SwitchBootstraps", "typeSwitch") => {
                self.type_switch_site(class, &b.args)
            }
            // @show
            (owner, name) => Err(Error::Unsupported(format!(
                "the bootstrap method {owner}.{name}"
            ))),
        }
    }

    /// The error the JDK's `BootstrapMethodInvoker` makes of an exception
    /// that a bootstrap method throws: a `BootstrapMethodError` with the
    /// exception as its cause.
    fn bootstrap_failure(&mut self, error: Error) -> Error {
        let Error::Exception { class, message } = error else { return error };
        let m = "bootstrap method initialization exception";
        if self.boot_dirs == 0 {
            return linkage("java/lang/BootstrapMethodError", m.to_string());
        }
        let wrapped = (|| {
            let cause = self.exception(class, message)?;
            self.heap.roots.push(cause);
            let error =
                self.exception("java/lang/BootstrapMethodError", Some(m.into()));
            let cause = self.heap.roots.pop().expect("pushed above");
            let (error, throwable) = (error?, self.load("java/lang/Throwable")?);
            let cause = Value::Ref(cause);
            self.put(&throwable, error, "cause", "Ljava/lang/Throwable;", cause)?;
            self.thrown(error)
        })();
        wrapped.unwrap_or_else(|failure| failure)
    }
    // @end
}

// @listing concat-site
/// The call site of `makeConcatWithConstants`, from its recipe: in the
/// recipe, `\u{1}` is the next argument, `\u{2}` the next constant among
/// the other static arguments, and anything else is text. A recipe that
/// doesn't fit the descriptor or the constants is a `StringConcatException`
/// with the JDK's message, which `link` wraps.
fn concat_site(
    pool: &ConstantPool,
    statics: &[u16],
    descriptor: MethodDescriptor,
) -> Result<CallSite> {
    let error = |m: String| {
        let class = "java/lang/invoke/StringConcatException";
        Error::Exception { class, message: Some(m) }
    };
    let params = descriptor.params.len();
    let arguments = |wanted: usize| {
        error(format!(
            "Mismatched number of concat arguments: recipe wants {wanted} \
             arguments, but signature provides {params}"
        ))
    };
    let Some((&recipe, constants)) = statics.split_first() else {
        // javac always passes a recipe, so this text is carafe's own.
        return Err(error("no recipe".to_string()));
    };
    let mismatch = |wanted: usize| {
        error(format!(
            "Mismatched number of concat constants: recipe wants {wanted} \
             constants, but only {} are passed",
            constants.len()
        ))
    };
    let (mut pieces, mut text, mut arg, mut used) = (Vec::new(), Vec::new(), 0, 0);
    for unit in string(pool, recipe)? {
        match unit {
            1 if arg == params => return Err(arguments(arg)),
            1 => {
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(Piece::Arg(arg));
                arg += 1;
            }
            2 => match constants.get(used) {
                Some(&constant) => {
                    text.extend(string(pool, constant)?);
                    used += 1;
                }
                None => return Err(mismatch(used)),
            },
            unit => text.push(unit),
        }
    }
    if arg != params {
        return Err(arguments(arg));
    }
    if used < constants.len() {
        return Err(mismatch(used));
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    Ok(CallSite::Concat { pieces, args: descriptor.params })
}

/// The UTF-16 code units of the `String` constant `index`.
fn string(pool: &ConstantPool, index: u16) -> Result<Vec<u16>> {
    let &Constant::String { utf8 } = pool.get(index)? else {
        return Err(Error::Unsupported(format!("a concatenation constant #{index}")));
    };
    Ok(match pool.get(utf8)? {
        Constant::Utf8(s) => s.encode_utf16().collect(),
        Constant::Utf16(units) => units.clone(),
        _ => return Err(Error::ClassFormat(format!("#{utf8} is not a Utf8"))),
    })
}
// @end

impl Vm {
    // @listing concat
    /// Concatenates the pieces, with each argument made a string by
    /// `value_of`. The arguments stay on the operand stack, as roots, until
    /// the end.
    fn concat(&mut self, pieces: &[Piece], args: &[FieldType]) -> Result<()> {
        let at = self.frames.len() - 1;
        let slots: usize = args.iter().map(|t| t.slots()).sum();
        let Some(base) = self.frames[at].stack.len().checked_sub(slots) else {
            return Err(Error::Verify("invokedynamic underflows the stack".into()));
        };
        let mut offsets = Vec::with_capacity(args.len());
        let mut slot = base;
        for ty in args {
            offsets.push(slot);
            slot += ty.slots();
        }
        let mut units = Vec::new();
        for piece in pieces {
            match *piece {
                Piece::Text(ref text) => units.extend_from_slice(text),
                Piece::Arg(n) => {
                    let value = self.frames[at].stack[offsets[n]];
                    units.extend(self.value_of(&args[n], value)?);
                }
            }
        }
        self.frames[at].stack.truncate(base);
        let result = self.new_string(&units)?;
        self.frames[at].push(Value::Ref(result))
    }

    /// A value of type `ty` as text, made by the JDK's `String.valueOf` for
    /// the type, which produces the characters JLS 15.18.1 specifies, as the
    /// JDK's own concatenation does: `valueOf(int)` for a `byte` or a
    /// `short`, and `valueOf(Object)` for any reference.
    pub(crate) fn value_of(
        &mut self,
        ty: &FieldType,
        value: Value,
    ) -> Result<Vec<u16>> {
        let descriptor = match ty {
            FieldType::Object(_) | FieldType::Array(_) => {
                "(Ljava/lang/Object;)Ljava/lang/String;"
            }
            FieldType::Byte | FieldType::Short | FieldType::Int => {
                "(I)Ljava/lang/String;"
            }
            FieldType::Boolean => "(Z)Ljava/lang/String;",
            FieldType::Char => "(C)Ljava/lang/String;",
            FieldType::Long => "(J)Ljava/lang/String;",
            FieldType::Float => "(F)Ljava/lang/String;",
            FieldType::Double => "(D)Ljava/lang/String;",
        };
        let string = self.load("java/lang/String")?;
        let Some(Value::Ref(s)) =
            self.invoke(&string, "valueOf", descriptor, &[value])?
        else {
            unreachable!("valueOf returns a String")
        };
        self.string_units(s)
    }
    // @end
}
