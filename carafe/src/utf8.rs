//! Modified UTF-8, the encoding of every string in a class file.

use crate::error::{Error, Result};

// @listing modified-utf8
/// Decodes modified UTF-8 (JVMS 4.4.7) into UTF-16 code units. Each group of
/// one, two or three bytes is exactly one code unit.
pub fn decode_modified_utf8(bytes: &[u8]) -> Result<Vec<u16>> {
    let mut units = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i] as u16;
        let (unit, len) = match b {
            0x01..=0x7f => (b, 1),
            0xc0..=0xdf => ((b & 0x1f) << 6 | low_six(bytes, i + 1)?, 2),
            0xe0..=0xef => {
                let high = (b & 0x0f) << 12 | low_six(bytes, i + 1)? << 6;
                (high | low_six(bytes, i + 2)?, 3)
            }
            _ => return Err(bad_utf8(i)),
        };
        units.push(unit);
        i += len;
    }
    Ok(units)
}
// @end

// @listing low-six
/// The low six bits of the continuation byte at `i`, which must be `10xxxxxx`.
fn low_six(bytes: &[u8], i: usize) -> Result<u16> {
    match bytes.get(i) {
        Some(&b) if b & 0xc0 == 0x80 => Ok((b & 0x3f) as u16),
        _ => Err(bad_utf8(i)),
    }
}

fn bad_utf8(i: usize) -> Error {
    Error::ClassFormat(format!(
        "malformed modified UTF-8 at byte {i} of a Utf8 entry"
    ))
}
// @end
