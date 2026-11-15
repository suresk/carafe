//! Reading a class file front to back.

use crate::error::{Error, Result};

// @listing reader
/// A cursor over the bytes of a class file. Every read checks that the bytes
/// are there, so a truncated file is an error and never a panic.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
// @end

impl<'a> Reader<'a> {
    // @listing reader-take
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes, at: 0 }
    }

    /// The offset of the next byte to be read.
    pub fn offset(&self) -> usize {
        self.at
    }

    /// The next `n` bytes, or `ClassFormat` if the file ends first.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let rest = &self.bytes[self.at..];
        let Some(taken) = rest.get(..n) else {
            return Err(Error::ClassFormat(format!(
                "truncated class file: {n} bytes needed at offset {}, {} left",
                self.at,
                rest.len()
            )));
        };
        self.at += n;
        Ok(taken)
    }
    // @end

    // @listing reader-integers
    /// The JVMS's `u1`, `u2` and `u4`: unsigned, big-endian.
    pub fn u1(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u2(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub fn u4(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().expect("take returned N bytes"))
    }
    // @end
}
