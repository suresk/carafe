//! Chapter 20: Just enough reflection.

use crate::class_file::ClassFile;
use crate::descriptor::FieldType;
use std::collections::BTreeSet;

// @listing modifier-string
/// The modifiers in `flags` as `java.lang.reflect.Modifier.toString`
/// writes them, in the order the JLS recommends.
pub fn modifier_string(flags: u16) -> String {
    const NAMES: [(u16, &str); 12] = [
        (0x0001, "public"),
        (0x0004, "protected"),
        (0x0002, "private"),
        (0x0400, "abstract"),
        (0x0008, "static"),
        (0x0010, "final"),
        (0x0080, "transient"),
        (0x0040, "volatile"),
        (0x0020, "synchronized"),
        (0x0100, "native"),
        (0x0800, "strictfp"),
        (0x0200, "interface"),
    ];
    let names =
        NAMES.iter().filter(|(bit, _)| flags & bit != 0).map(|(_, name)| *name);
    names.collect::<Vec<_>>().join(" ")
}
// @end

// @listing signature-classes
/// The classes that the descriptors of a class's methods name, other than
/// constructors and `<clinit>`: what `getDeclaredMethods` must load to make
/// the parameter and return types' mirrors. An array names its element's class.
pub fn signature_classes(file: &ClassFile) -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    for m in file.methods.iter().filter(|m| !m.name.starts_with('<')) {
        for ty in m.ty.params.iter().chain(&m.ty.ret) {
            let mut ty = ty;
            while let FieldType::Array(element) = ty {
                ty = element;
            }
            if let FieldType::Object(name) = ty {
                classes.insert(name.clone());
            }
        }
    }
    classes
}
// @end
