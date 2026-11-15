//! Chapter 11: virtual dispatch.

use crate::class::Class;
use crate::flags::AccessFlags;
use crate::interpreter::select;
use std::rc::Rc;

// @listing vtable
/// The class's virtual methods, as HotSpot's vtable orders them: the
/// superclass's entries first, each replaced by the method that overrides it
/// here, then the class's new methods. Each entry is (name + descriptor,
/// the class whose method runs).
pub fn vtable(class: &Rc<Class>) -> Vec<(String, String)> {
    let mut table = match &class.superclass {
        Some(superclass) => vtable(superclass),
        None => Vec::new(),
    };
    for (index, method) in class.file.methods.iter().enumerate() {
        let flags = method.access_flags;
        let skip = AccessFlags::PRIVATE | AccessFlags::STATIC;
        if flags.has(skip) || method.name.starts_with('<') {
            continue;
        }
        let key = format!("{}{}", method.name, method.descriptor);
        let owner = class.file.this_class.clone();
        // An entry this method overrides is replaced; otherwise it is new.
        let replaced = table
            .iter_mut()
            .find(|(k, holder)| *k == key && overridden(class, index, holder, &key));
        match replaced {
            Some(entry) => entry.1 = owner,
            None => table.push((key, owner)),
        }
    }
    table
}

/// Whether method `index` of `class` overrides the method `key` of its
/// superclass `holder`, by asking the selection rules.
fn overridden(class: &Rc<Class>, index: usize, holder: &str, key: &str) -> bool {
    let mut c = class.superclass.clone();
    while let Some(current) = c {
        if current.file.this_class == holder {
            let (name, descriptor) = key.split_at(key.find('(').unwrap_or(key.len()));
            let Ok(mr) = current.method(name, descriptor) else { return false };
            return select(class, &current, mr)
                .is_some_and(|(c, m)| Rc::ptr_eq(&c, class) && m == index);
        }
        c = current.superclass.clone();
    }
    false
}
// @end
