//! `invokedynamic`: bootstrap methods, call sites and string concatenation.

use crate::class::Class;
use crate::class_file::ClassFile;
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::error::{Error, Result};
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::pool::{Constant, ConstantPool};
use crate::reader::Reader;
use crate::value::Value;
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
#[derive(Debug)]
pub enum CallSite {
    /// String concatenation: the pieces of the result, in order, and the
    /// types of the arguments the instruction pops.
    Concat { pieces: Vec<Piece>, args: Vec<FieldType> },
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
    /// runs, and then does what the call site says.
    pub(crate) fn invoke_dynamic(&mut self, index: u16, pc: u16) -> Result<()> {
        let frame = self.frames.last().expect("a frame");
        let (class, key) = (frame.class.clone(), (frame.method, pc));
        let site = class.call_sites.borrow().get(&key).cloned();
        let site = match site {
            Some(site) => site,
            None => {
                let site = Rc::new(link(&class, index)?);
                class.call_sites.borrow_mut().insert(key, site.clone());
                site
            }
        };
        match &*site {
            CallSite::Concat { pieces, args } => self.concat(pieces, args),
        }
    }
    // @end
}

// @listing link
/// Links the call site of the `InvokeDynamic` constant `index` by running
/// its bootstrap method, which carafe knows by name.
fn link(class: &Class, index: u16) -> Result<CallSite> {
    let pool = &class.file.constant_pool;
    let &Constant::InvokeDynamic { bootstrap, name_and_type } = pool.get(index)?
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
        }
        (owner, name) => {
            Err(Error::Unsupported(format!("the bootstrap method {owner}.{name}")))
        }
    }
}
// @end

// @listing concat-site
/// The call site of `makeConcatWithConstants`, from its recipe: in the
/// recipe, `\u{1}` is the next argument, `\u{2}` the next constant among
/// the other static arguments, and anything else is text.
fn concat_site(
    pool: &ConstantPool,
    statics: &[u16],
    descriptor: MethodDescriptor,
) -> Result<CallSite> {
    // HotSpot's error, whose cause would be a StringConcatException.
    let error = |m: String| linkage("java/lang/BootstrapMethodError", m);
    let Some((&recipe, constants)) = statics.split_first() else {
        return Err(error("no recipe".to_string()));
    };
    let mut constants = constants.iter();
    let (mut pieces, mut text, mut arg) = (Vec::new(), Vec::new(), 0);
    for unit in string(pool, recipe)? {
        match unit {
            1 => {
                pieces.push(Piece::Text(std::mem::take(&mut text)));
                pieces.push(Piece::Arg(arg));
                arg += 1;
            }
            2 => match constants.next() {
                Some(&constant) => text.extend(string(pool, constant)?),
                None => return Err(error("too few constants".to_string())),
            },
            unit => text.push(unit),
        }
    }
    pieces.push(Piece::Text(text));
    pieces.retain(|p| *p != Piece::Text(Vec::new()));
    if arg != descriptor.params.len() {
        let m = format!(
            "Mismatched number of concat arguments: recipe wants {arg} \
                         arguments, but signature provides {}",
            descriptor.params.len()
        );
        return Err(error(m));
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
    /// Concatenates the pieces, with each argument made a string by the
    /// `String.valueOf` for its type, as the JDK's own strategy does. The
    /// arguments stay on the operand stack, as roots, until the end.
    fn concat(&mut self, pieces: &[Piece], args: &[FieldType]) -> Result<()> {
        let at = self.frames.len() - 1;
        let slots: usize = args.iter().map(|t| t.slots()).sum();
        let Some(base) = self.frames[at].stack.len().checked_sub(slots) else {
            return Err(Error::Verify("invokedynamic underflows the stack".into()));
        };
        let string = self.load("java/lang/String")?;
        let mut units = Vec::new();
        for piece in pieces {
            match *piece {
                Piece::Text(ref text) => units.extend_from_slice(text),
                Piece::Arg(n) => {
                    let slot =
                        base + args[..n].iter().map(|t| t.slots()).sum::<usize>();
                    let value = self.frames[at].stack[slot];
                    let descriptor = match &args[n] {
                        FieldType::Object(_) | FieldType::Array(_) => {
                            "Ljava/lang/Object;"
                        }
                        FieldType::Byte | FieldType::Short => "I",
                        t if *t == FieldType::Boolean => "Z",
                        other => &other.descriptor(),
                    };
                    let descriptor = format!("({descriptor})Ljava/lang/String;");
                    let Some(Value::Ref(s)) =
                        self.invoke(&string, "valueOf", &descriptor, &[value])?
                    else {
                        unreachable!("valueOf returns a String")
                    };
                    units.extend(self.string_units(s)?);
                }
            }
        }
        let frame = &mut self.frames[at];
        frame.stack.truncate(base);
        let result = self.new_string(&units)?;
        self.frames[at].push(Value::Ref(result))
    }
    // @end
}
