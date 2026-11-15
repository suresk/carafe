//! carafe: the Java virtual machine you build in *Build a JVM in Rust*.

mod challenges;
mod class_file;
mod error;
mod jdk;
mod pool;
mod reader;
mod utf8;

pub use class_file::{ClassFile, MAGIC, MAX_MAJOR, MIN_MAJOR};
pub use error::{Error, Result};
pub use jdk::{JDK_VERSION, Jdk};
pub use pool::{Constant, ConstantPool, MemberRef};
pub use reader::Reader;
pub use utf8::decode_modified_utf8;

pub use challenges::ch01::{
    check_pool, encode_modified_utf8, tag_counts, write_pool,
};
