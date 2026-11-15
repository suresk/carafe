//! carafe: the Java virtual machine you build in *Build a JVM in Rust*.

mod attribute;
mod bytecode;
mod challenges;
mod class_file;
mod descriptor;
mod disassemble;
mod error;
mod flags;
mod jdk;
mod member;
pub mod opcode;
mod pool;
mod reader;
mod utf8;

pub use attribute::{Attribute, Code, Handler};
pub use bytecode::{Instruction, Operand};
pub use class_file::{ClassFile, MAGIC, MAX_MAJOR, MIN_MAJOR};
pub use descriptor::{FieldType, MethodDescriptor};
pub use error::{Error, Result};
pub use flags::{AccessFlags, Flagged};
pub use jdk::{JDK_VERSION, Jdk};
pub use member::{Field, Method};
pub use pool::{Constant, ConstantPool, MemberRef};
pub use reader::Reader;
pub use utf8::decode_modified_utf8;

pub use challenges::ch01::{
    check_pool, encode_modified_utf8, tag_counts, write_pool,
};
pub use challenges::ch02::{check_flags, declaration, line_number};
pub use challenges::ch03::{encode_bytecode, exception_table, unreachable};
