//! Chapter 13: interfaces and casts.

use crate::casts::modules;
use crate::class::Class;
use crate::flags::AccessFlags;
use crate::resolve::maximally_specific;
use std::rc::Rc;

// @listing illegal-access
/// Chapter 8's `IllegalAccessError` message for a use by `from` of a member
/// of `owner`, completed with HotSpot's note on their modules and loaders.
pub fn illegal_access_message(message: &str, from: &Class, owner: &Class) -> String {
    format!("{message} ({})", modules(from, owner))
}
// @end

// @listing conflicting
/// HotSpot's message when `receiver` inherits more than one default method
/// for `name` with `descriptor` and overrides none, or `None` if it doesn't.
pub fn conflicting_defaults(
    receiver: &Rc<Class>,
    name: &str,
    descriptor: &str,
) -> Option<String> {
    let specific = maximally_specific(receiver, name, descriptor);
    let defaults: Vec<String> = specific
        .iter()
        .filter(|(c, m)| !c.file.methods[*m].access_flags.has(AccessFlags::ABSTRACT))
        .map(|(c, _)| format!("{}.{name}", c.file.this_class.replace('/', ".")))
        .collect();
    (defaults.len() > 1)
        .then(|| format!("Conflicting default methods: {}", defaults.join(" ")))
}
// @end
