//! Printing instructions as `javap -c` does.

use crate::bytecode::{Instruction, Operand};
use crate::class_file::ClassFile;
use crate::error::Result;
use crate::opcode::MNEMONICS;
use crate::pool::Constant;

impl Instruction {
    // @listing disassemble
    /// The instruction's name as `javap` prints it: `iload`, or `iload_w`
    /// after `wide`.
    pub fn mnemonic(&self) -> String {
        let name = MNEMONICS[usize::from(self.opcode)].to_ascii_lowercase();
        if self.wide { name + "_w" } else { name }
    }

    /// The instruction as `javap -c` prints it, with the constants it uses
    /// described after `//`. A switch takes several lines.
    pub fn disassemble(&self, class: &ClassFile) -> Result<String> {
        let (operands, constant) = match &self.operand {
            Operand::None => (String::new(), None),
            Operand::Value(v) => (v.to_string(), None),
            Operand::Local(n) => (n.to_string(), None),
            Operand::Target(pc) => (pc.to_string(), None),
            Operand::Iinc { local, delta } => (format!("{local}, {delta}"), None),
            Operand::Constant(i) => (format!("#{i}"), Some(*i)),
            Operand::Interface { index, count } => {
                (format!("#{index}, {count}"), Some(*index))
            }
            Operand::Dynamic(i) => (format!("#{i}, 0"), Some(*i)),
            Operand::ArrayType(t) => (array_type(*t).to_string(), None),
            Operand::MultiArray { index, dimensions } => {
                (format!("#{index}, {dimensions}"), Some(*index))
            }
            Operand::TableSwitch { default, low, targets } => {
                let high = *low as i64 + targets.len() as i64 - 1;
                let keys = (i64::from(*low)..).zip(targets.iter().copied());
                (switch(&format!("{low} to {high}"), keys, *default), None)
            }
            Operand::LookupSwitch { default, pairs } => {
                let keys = pairs.iter().map(|&(k, t)| (i64::from(k), t));
                (switch(&pairs.len().to_string(), keys, *default), None)
            }
        };
        let line = format!("{:>5}: {:<13} {operands}", self.pc, self.mnemonic());
        Ok(match constant {
            Some(i) => format!("{line:<40} // {}", describe(class, i)?),
            None => line.trim_end().to_string(),
        })
    }
    // @end
}

// @listing switch
/// The braces and lines of a switch: one line per key, then the default.
fn switch(
    header: &str,
    keys: impl Iterator<Item = (i64, u16)>,
    default: u16,
) -> String {
    let mut out = format!("{{ // {header}\n");
    for (key, target) in keys {
        out += &format!("{key:>18}: {target}\n");
    }
    out + &format!("{:>18}: {default}\n{:>8}", "default", "}")
}

/// The element type of `newarray` (JVMS Table 6.5.newarray-A).
fn array_type(code: u8) -> &'static str {
    match code {
        4 => "boolean",
        5 => "char",
        6 => "float",
        7 => "double",
        8 => "byte",
        9 => "short",
        10 => "int",
        11 => "long",
        _ => "?",
    }
}
// @end

// @listing describe-constant
/// A constant as `javap -c` describes it: its kind, then what it holds or
/// names. Members of the class itself are named without the class.
fn describe(class: &ClassFile, index: u16) -> Result<String> {
    let pool = &class.constant_pool;
    let constant = pool.get(index)?;
    let kind = match constant {
        Constant::Methodref { .. } => "Method",
        Constant::Fieldref { .. } => "Field",
        Constant::InterfaceMethodref { .. } => "InterfaceMethod",
        Constant::Class { .. } => "class",
        Constant::Integer(_) => "int",
        Constant::Float(_) => "float",
        Constant::Long(_) => "long",
        Constant::Double(_) => "double",
        other => other.describe().0,
    };
    let text = match constant {
        Constant::Integer(_)
        | Constant::Float(_)
        | Constant::Long(_)
        | Constant::Double(_) => constant.describe().1,
        _ => pool.comment(index, constant)?,
    };
    let own = format!("{}.", class.this_class);
    let text = match constant {
        Constant::Fieldref { .. }
        | Constant::Methodref { .. }
        | Constant::InterfaceMethodref { .. } => {
            text.strip_prefix(&own).unwrap_or(&text)
        }
        _ => &text,
    };
    Ok(format!("{kind} {text}"))
}
// @end
