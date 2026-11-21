//! Access flags (JVMS 4.1, 4.5 and 4.6).

// @listing access-flags
/// The access flags of a class, a field or a method: a set of bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessFlags(pub u16);

impl AccessFlags {
    pub const PUBLIC: u16 = 0x0001;
    pub const PRIVATE: u16 = 0x0002;
    pub const PROTECTED: u16 = 0x0004;
    pub const STATIC: u16 = 0x0008;
    pub const FINAL: u16 = 0x0010;
    pub const SUPER: u16 = 0x0020;
    pub const SYNCHRONIZED: u16 = 0x0020;
    pub const VOLATILE: u16 = 0x0040;
    pub const BRIDGE: u16 = 0x0040;
    pub const TRANSIENT: u16 = 0x0080;
    pub const VARARGS: u16 = 0x0080;
    pub const NATIVE: u16 = 0x0100;
    pub const INTERFACE: u16 = 0x0200;
    pub const ABSTRACT: u16 = 0x0400;
    pub const STRICT: u16 = 0x0800;
    pub const SYNTHETIC: u16 = 0x1000;
    pub const ANNOTATION: u16 = 0x2000;
    pub const ENUM: u16 = 0x4000;
    pub const MODULE: u16 = 0x8000;

    /// Whether any of the bits in `flags` is set.
    pub fn has(self, flags: u16) -> bool {
        self.0 & flags != 0
    }

    /// The flags as `javap` prints them: `(0x0009) ACC_PUBLIC, ACC_STATIC`.
    pub fn describe(self, of: Flagged) -> String {
        let names = NAMES.iter().map(|(bit, names)| (*bit, names[of as usize]));
        let set: Vec<String> = names
            .filter(|&(bit, name)| self.has(bit) && !name.is_empty())
            .map(|(_, name)| format!("ACC_{name}"))
            .collect();
        let bits = format!("({:#06x})", self.0);
        if set.is_empty() { bits } else { format!("{bits} {}", set.join(", ")) }
    }
}
// @end

// @listing flag-names
/// What a set of flags belongs to, which decides what some of its bits mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flagged {
    Class,
    Field,
    Method,
}

/// Each bit's name on a class, a field and a method, or "" where the bit has
/// no meaning (JVMS Tables 4.1-B, 4.5-A and 4.6-A).
const NAMES: [(u16, [&str; 3]); 16] = [
    (0x0001, ["PUBLIC", "PUBLIC", "PUBLIC"]),
    (0x0002, ["", "PRIVATE", "PRIVATE"]),
    (0x0004, ["", "PROTECTED", "PROTECTED"]),
    (0x0008, ["", "STATIC", "STATIC"]),
    (0x0010, ["FINAL", "FINAL", "FINAL"]),
    (0x0020, ["SUPER", "", "SYNCHRONIZED"]),
    (0x0040, ["", "VOLATILE", "BRIDGE"]),
    (0x0080, ["", "TRANSIENT", "VARARGS"]),
    (0x0100, ["", "", "NATIVE"]),
    (0x0200, ["INTERFACE", "", ""]),
    (0x0400, ["ABSTRACT", "", "ABSTRACT"]),
    (0x0800, ["", "", "STRICT"]),
    (0x1000, ["SYNTHETIC", "SYNTHETIC", "SYNTHETIC"]),
    (0x2000, ["ANNOTATION", "", ""]),
    (0x4000, ["ENUM", "ENUM", ""]),
    (0x8000, ["MODULE", "", ""]),
];
// @end
