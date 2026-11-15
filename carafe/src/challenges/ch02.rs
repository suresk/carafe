//! Chapter 2: fields, methods and attributes.

use crate::attribute::Code;
use crate::class_file::ClassFile;
use crate::error::{Error, Result};
use crate::flags::AccessFlags as F;
use crate::member::Method;
use crate::reader::Reader;

// @listing check-flags
/// Checks the combinations of access flags that JVMS 4.1, 4.5 and 4.6 forbid.
pub fn check_flags(class: &ClassFile) -> Result<()> {
    let flags = class.access_flags;
    let interface = flags.has(F::INTERFACE);
    if interface && (!flags.has(F::ABSTRACT) || flags.has(F::FINAL | F::ENUM)) {
        return bad(
            &class.this_class,
            "an interface that is final, an enum or not abstract",
        );
    }
    if flags.has(F::ANNOTATION) && !interface {
        return bad(&class.this_class, "an annotation that is not an interface");
    }
    if flags.has(F::FINAL) && flags.has(F::ABSTRACT) {
        return bad(&class.this_class, "both final and abstract");
    }
    let fields = class.fields.iter().map(|f| (&f.name, f.access_flags));
    let methods = class.methods.iter().map(|m| (&m.name, m.access_flags));
    for (name, flags) in fields.chain(methods) {
        if (flags.0 & (F::PUBLIC | F::PRIVATE | F::PROTECTED)).count_ones() > 1 {
            return bad(name, "more than one of public, private and protected");
        }
    }
    let forbidden = F::PRIVATE | F::STATIC | F::FINAL | F::SYNCHRONIZED | F::NATIVE;
    for method in &class.methods {
        let flags = method.access_flags;
        if flags.has(F::ABSTRACT) && flags.has(forbidden) {
            return bad(
                &method.name,
                "abstract and private, static, final, ... or native",
            );
        }
    }
    Ok(())
}

fn bad(what: &str, problem: &str) -> Result<()> {
    Err(Error::ClassFormat(format!("{what} is {problem}")))
}
// @end

// @listing line-number
/// The source line of the instruction at `pc`, from the `LineNumberTable`
/// attributes of `code` (JVMS 4.7.12).
pub fn line_number(code: &Code, pc: u16) -> Result<Option<u16>> {
    let mut best: Option<(u16, u16)> = None; // (start_pc, line)
    for attribute in code.attributes.iter().filter(|a| a.name == "LineNumberTable") {
        let mut r = Reader::new(&attribute.info);
        for _ in 0..r.u2()? {
            let (start_pc, line) = (r.u2()?, r.u2()?);
            if start_pc <= pc && best.is_none_or(|(start, _)| start_pc > start) {
                best = Some((start_pc, line));
            }
        }
    }
    Ok(best.map(|(_, line)| line))
}
// @end

// @listing declaration
/// A method as `javap` declares it: `public static void main(java.lang.String[])`.
/// Generic types (the `Signature` attribute) and `throws` are left out.
pub fn declaration(class: &ClassFile, method: &Method) -> String {
    let flags = method.access_flags;
    let interface = class.access_flags.has(F::INTERFACE);
    let default = interface && !flags.has(F::ABSTRACT | F::STATIC | F::PRIVATE);
    let modifiers = [
        (flags.has(F::PUBLIC), "public "),
        (flags.has(F::PROTECTED), "protected "),
        (flags.has(F::PRIVATE), "private "),
        (flags.has(F::ABSTRACT), "abstract "),
        (flags.has(F::STATIC), "static "),
        (flags.has(F::FINAL), "final "),
        (flags.has(F::SYNCHRONIZED), "synchronized "),
        (flags.has(F::NATIVE), "native "),
        (default, "default "),
    ];
    let mut out: String = modifiers.iter().filter(|m| m.0).map(|m| m.1).collect();
    if method.name == "<clinit>" {
        return out + "{}";
    }
    if method.name == "<init>" {
        out += &class.this_class.replace('/', ".");
    } else {
        match &method.ty.ret {
            Some(ty) => out += &ty.to_string(),
            None => out += "void",
        }
        out += &format!(" {}", method.name);
    }
    let params: Vec<String> =
        method.ty.params.iter().map(|p| p.to_string()).collect();
    out + &format!("({})", params.join(", "))
}
// @end
