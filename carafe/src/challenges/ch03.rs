//! Chapter 3: reading bytecode.

use crate::attribute::Code;
use crate::bytecode::{Instruction, Operand};
use crate::class_file::ClassFile;
use crate::error::Result;
use crate::opcode::*;
use std::collections::BTreeSet;

// @listing encode
/// Writes decoded instructions back as bytecode.
pub fn encode_bytecode(instructions: &[Instruction]) -> Vec<u8> {
    let mut out = Vec::new();
    for i in instructions {
        if i.wide {
            out.push(WIDE);
        }
        out.push(i.opcode);
        let offset = |to: u16| (i32::from(to) - i32::from(i.pc)).to_be_bytes();
        match &i.operand {
            Operand::None => {}
            Operand::Value(v) if i.opcode == BIPUSH => out.push(*v as u8),
            Operand::Value(v) => out.extend(v.to_be_bytes()),
            Operand::Local(n) | Operand::Constant(n) if narrow(i) => {
                out.push(*n as u8)
            }
            Operand::Local(n) | Operand::Constant(n) => out.extend(n.to_be_bytes()),
            Operand::Iinc { local, delta } if i.wide => {
                out.extend(local.to_be_bytes());
                out.extend(delta.to_be_bytes());
            }
            Operand::Iinc { local, delta } => {
                out.extend([*local as u8, *delta as u8])
            }
            Operand::Target(to) if i.opcode == GOTO_W => out.extend(offset(*to)),
            Operand::Target(to) => out.extend(&offset(*to)[2..]),
            Operand::Interface { index, count } => {
                out.extend(index.to_be_bytes());
                out.extend([*count, 0]);
            }
            Operand::Dynamic(index) => {
                out.extend([&index.to_be_bytes()[..], &[0, 0]].concat())
            }
            Operand::ArrayType(t) => out.push(*t),
            Operand::MultiArray { index, dimensions } => {
                out.extend(index.to_be_bytes());
                out.push(*dimensions);
            }
            Operand::TableSwitch { default, low, targets } => {
                out.resize(out.len().next_multiple_of(4), 0);
                out.extend(offset(*default));
                out.extend(low.to_be_bytes());
                out.extend((low + targets.len() as i32 - 1).to_be_bytes());
                targets.iter().for_each(|&t| out.extend(offset(t)));
            }
            Operand::LookupSwitch { default, pairs } => {
                out.resize(out.len().next_multiple_of(4), 0);
                out.extend(offset(*default));
                out.extend((pairs.len() as u32).to_be_bytes());
                for &(key, t) in pairs {
                    out.extend(key.to_be_bytes());
                    out.extend(offset(t));
                }
            }
        }
    }
    out
}

/// Whether the instruction's index operand is one byte: `ldc`, and the
/// loads and stores without `wide`.
fn narrow(i: &Instruction) -> bool {
    i.opcode == LDC || (!i.wide && matches!(i.operand, Operand::Local(_)))
}
// @end

// @listing exception-table
/// The exception table of `code` as `javap -c` prints it.
pub fn exception_table(class: &ClassFile, code: &Code) -> Result<Vec<String>> {
    if code.handlers.is_empty() {
        return Ok(Vec::new());
    }
    let mut lines =
        vec!["Exception table:".to_string(), "from to target type".to_string()];
    for h in &code.handlers {
        let caught = match h.catch_type {
            0 => "any".to_string(),
            index => format!("Class {}", class.constant_pool.class_name(index)?),
        };
        let (from, to, target) = (h.start_pc, h.end_pc, h.handler_pc);
        lines.push(format!("{from:>5} {to:>5} {target:>5}   {caught}"));
    }
    Ok(lines)
}
// @end

// @listing unreachable
/// The pcs of the instructions that no path of execution reaches, starting
/// from pc 0 and from each exception handler.
pub fn unreachable(code: &Code) -> Result<Vec<u16>> {
    let instructions = code.decode()?;
    let index_of = |pc: u16| instructions.binary_search_by_key(&pc, |i| i.pc).ok();
    let mut seen = BTreeSet::new();
    let mut work: Vec<u16> = code.handlers.iter().map(|h| h.handler_pc).collect();
    work.push(0);
    while let Some(pc) = work.pop() {
        let Some(at) = index_of(pc) else { continue };
        if !seen.insert(pc) {
            continue;
        }
        let i = &instructions[at];
        work.extend(i.targets());
        let stops = matches!(
            i.opcode,
            GOTO | GOTO_W | TABLESWITCH | LOOKUPSWITCH | IRETURN..=RETURN | ATHROW
        );
        if !stops && let Some(next) = instructions.get(at + 1) {
            work.push(next.pc);
        }
    }
    Ok(instructions.iter().map(|i| i.pc).filter(|pc| !seen.contains(pc)).collect())
}
// @end
