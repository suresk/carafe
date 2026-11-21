//! Decoding bytecode into instructions (JVMS 4.9.1 and 6.5).

use crate::attribute::Code;
use crate::error::{Error, Result};
use crate::opcode::*;
use crate::reader::Reader;

// @listing instruction
/// One instruction: where it starts, its opcode, and its operands decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub pc: u16,
    pub opcode: u8,
    /// Whether a `wide` prefix widened the operands (`iload_w`, `iinc_w`).
    pub wide: bool,
    pub operand: Operand,
}

/// The operands of an instruction, in one of the JVM's formats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    None,
    /// `bipush` and `sipush`: the value to push.
    Value(i16),
    /// A local variable slot, for the loads and stores that take one.
    Local(u16),
    /// A constant pool index: `ldc`, field and method access, `new` and others.
    Constant(u16),
    /// The pc a branch goes to, already added to the branch's own pc.
    Target(u16),
    Iinc {
        local: u16,
        delta: i16,
    },
    /// `invokeinterface`: the method, and the slots its arguments fill.
    Interface {
        index: u16,
        count: u8,
    },
    /// `invokedynamic`: the call site's `InvokeDynamic` constant.
    Dynamic(u16),
    /// `newarray`: the element type, 4 (`boolean`) to 11 (`long`).
    ArrayType(u8),
    MultiArray {
        index: u16,
        dimensions: u8,
    },
    TableSwitch {
        default: u16,
        low: i32,
        targets: Vec<u16>,
    },
    LookupSwitch {
        default: u16,
        pairs: Vec<(i32, u16)>,
    },
}
// @end

// @listing decode
impl Code {
    /// Decodes the bytecode into instructions, and checks that every branch
    /// and every exception handler starts at an instruction.
    pub fn decode(&self) -> Result<Vec<Instruction>> {
        let mut r = Reader::new(&self.bytecode);
        let mut instructions = Vec::new();
        while r.left() > 0 {
            instructions.push(instruction(&mut r)?);
        }
        let mut starts = vec![false; self.bytecode.len()];
        for i in &instructions {
            starts[usize::from(i.pc)] = true;
        }
        let starts_at = |pc: u16| starts.get(usize::from(pc)) == Some(&true);
        for i in &instructions {
            if let Some(&bad) = i.targets().iter().find(|&&t| !starts_at(t)) {
                return Err(not_a_start(&format!("the branch at {}", i.pc), bad));
            }
        }
        for h in &self.handlers {
            // end_pc is exclusive, so it may also be the end of the code.
            let at_end = usize::from(h.end_pc) == self.bytecode.len();
            let ends = (!at_end).then_some(h.end_pc);
            for pc in [h.start_pc, h.handler_pc].into_iter().chain(ends) {
                if !starts_at(pc) {
                    return Err(not_a_start("an exception handler", pc));
                }
            }
        }
        Ok(instructions)
    }
}

fn not_a_start(what: &str, pc: u16) -> Error {
    let m =
        format!("{what} refers to {pc}, which is not the start of an instruction");
    Error::ClassFormat(m)
}
// @end

// @listing instruction-decode
/// Decodes the instruction at the reader's position.
fn instruction(r: &mut Reader) -> Result<Instruction> {
    let pc = r.offset();
    let opcode = r.u1()?;
    let operand = match opcode {
        BIPUSH => Operand::Value(r.u1()? as i8 as i16),
        SIPUSH => Operand::Value(r.u2()? as i16),
        LDC => Operand::Constant(r.u1()?.into()),
        LDC_W
        | LDC2_W
        | GETSTATIC..=INVOKESTATIC
        | NEW
        | ANEWARRAY
        | CHECKCAST
        | INSTANCEOF => Operand::Constant(r.u2()?),
        ILOAD..=ALOAD | ISTORE..=ASTORE => Operand::Local(r.u1()?.into()),
        IINC => Operand::Iinc { local: r.u1()?.into(), delta: r.u1()? as i8 as i16 },
        IFEQ..=GOTO | IFNULL | IFNONNULL => {
            Operand::Target(target(pc, r.u2()? as i16 as i32)?)
        }
        GOTO_W => Operand::Target(target(pc, r.u4()? as i32)?),
        TABLESWITCH => table_switch(r, pc)?,
        LOOKUPSWITCH => lookup_switch(r, pc)?,
        INVOKEINTERFACE => {
            let (index, count, zero) = (r.u2()?, r.u1()?, r.u1()?);
            if count == 0 || zero != 0 {
                return Err(bad(pc, opcode, "needs a count and then a zero byte"));
            }
            Operand::Interface { index, count }
        }
        INVOKEDYNAMIC => {
            let (index, zeros) = (r.u2()?, r.u2()?);
            if zeros != 0 {
                return Err(bad(pc, opcode, "needs two zero bytes after its index"));
            }
            Operand::Dynamic(index)
        }
        NEWARRAY => match r.u1()? {
            atype @ 4..=11 => Operand::ArrayType(atype),
            atype => return Err(bad(pc, opcode, &format!("has array type {atype}"))),
        },
        MULTIANEWARRAY => match (r.u2()?, r.u1()?) {
            (_, 0) => return Err(bad(pc, opcode, "has 0 dimensions")),
            (index, dimensions) => Operand::MultiArray { index, dimensions },
        },
        WIDE => return wide(r, pc),
        JSR | RET | JSR_W => {
            return Err(bad(pc, opcode, "is not allowed after Java 6"));
        }
        _ if MNEMONICS[usize::from(opcode)].is_empty() => {
            return Err(bad(pc, opcode, "is not an instruction"));
        }
        _ => Operand::None,
    };
    let pc = pc as u16; // the code is at most 65,535 bytes (Code::parse)
    Ok(Instruction { pc, opcode, wide: false, operand })
}

fn bad(pc: usize, opcode: u8, problem: &str) -> Error {
    Error::ClassFormat(format!("the opcode {opcode:#04x} at {pc} {problem}"))
}

/// The pc that a branch at `pc` with the offset `offset` goes to.
fn target(pc: usize, offset: i32) -> Result<u16> {
    let to = pc as i64 + i64::from(offset);
    u16::try_from(to).map_err(|_| {
        Error::ClassFormat(format!(
            "the branch at {pc} goes to {to}, outside the code"
        ))
    })
}

/// `wide` and the instruction it modifies, which takes a two-byte local
/// variable index, and for `iinc` a two-byte increment.
fn wide(r: &mut Reader, pc: usize) -> Result<Instruction> {
    let opcode = r.u1()?;
    let operand = match opcode {
        ILOAD..=ALOAD | ISTORE..=ASTORE => Operand::Local(r.u2()?),
        IINC => Operand::Iinc { local: r.u2()?, delta: r.u2()? as i16 },
        RET => return Err(bad(pc + 1, opcode, "is not allowed after Java 6")),
        _ => return Err(bad(pc + 1, opcode, "cannot follow wide")),
    };
    Ok(Instruction { pc: pc as u16, opcode, wide: true, operand })
}
// @end

// @listing switches
/// `tableswitch`: padding to a multiple of four bytes from the start of the
/// code, then the default, the lowest and highest keys, and one offset per key.
fn table_switch(r: &mut Reader, pc: usize) -> Result<Operand> {
    skip_padding(r)?;
    let default = target(pc, r.u4()? as i32)?;
    let (low, high) = (r.u4()? as i32, r.u4()? as i32);
    if low > high {
        let m = format!("the tableswitch at {pc} has keys from {low} to {high}");
        return Err(Error::ClassFormat(m));
    }
    let mut targets = Vec::new();
    for _ in low..=high {
        targets.push(target(pc, r.u4()? as i32)?);
    }
    Ok(Operand::TableSwitch { default, low, targets })
}

/// `lookupswitch`: the same padding, the default, then (key, offset) pairs
/// sorted by key.
fn lookup_switch(r: &mut Reader, pc: usize) -> Result<Operand> {
    skip_padding(r)?;
    let default = target(pc, r.u4()? as i32)?;
    let count = r.u4()?;
    let mut pairs: Vec<(i32, u16)> = Vec::new();
    for _ in 0..count {
        let key = r.u4()? as i32;
        if pairs.last().is_some_and(|&(last, _)| last >= key) {
            let m = format!("the keys of the lookupswitch at {pc} are not sorted");
            return Err(Error::ClassFormat(m));
        }
        pairs.push((key, target(pc, r.u4()? as i32)?));
    }
    Ok(Operand::LookupSwitch { default, pairs })
}

/// Skips the zero to three bytes that bring the reader to a multiple of four.
fn skip_padding(r: &mut Reader) -> Result<()> {
    while !r.offset().is_multiple_of(4) {
        r.u1()?;
    }
    Ok(())
}
// @end

impl Instruction {
    // @listing targets
    /// Every pc this instruction can branch to.
    pub fn targets(&self) -> Vec<u16> {
        match &self.operand {
            Operand::Target(t) => vec![*t],
            Operand::TableSwitch { default, targets, .. } => {
                [*default].into_iter().chain(targets.iter().copied()).collect()
            }
            Operand::LookupSwitch { default, pairs } => {
                [*default].into_iter().chain(pairs.iter().map(|p| p.1)).collect()
            }
            _ => Vec::new(),
        }
    }
    // @end
}
