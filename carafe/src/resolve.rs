//! Resolving symbolic references to classes, fields and methods
//! (JVMS 5.4.3), with the access checks of JVMS 5.4.4.

use crate::cache::Resolved;
use crate::casts::is_subtype;
use crate::class::Class;
use crate::descriptor::{FieldType, MethodDescriptor};
use crate::error::Result;
use crate::flags::AccessFlags;
use crate::interpreter::Vm;
use crate::loader::linkage;
use crate::member::Field;
use crate::pool::Constant;
use crate::reader::Reader;
use std::rc::Rc;

impl Vm {
    // @listing resolve-class
    /// Resolves a reference from `from` to the class `name`: loads it, and
    /// checks that `from` may refer to it.
    pub fn resolve_class(&mut self, from: &Class, name: &str) -> Result<Rc<Class>> {
        let class = self.load(name)?;
        // @hide
        // An array class is as accessible as its element class.
        let name = name.trim_start_matches('[');
        let name =
            name.strip_prefix('L').and_then(|n| n.strip_suffix(';')).unwrap_or(name);
        // @show
        let public = class.file.access_flags.has(AccessFlags::PUBLIC);
        if !public && package(name) != package(&from.file.this_class) {
            let m = format!(
                "failed to access class {} from class {}",
                dotted(name),
                dotted(&from.file.this_class)
            );
            return Err(linkage("java/lang/IllegalAccessError", m));
        }
        Ok(class)
    }
    // @end

    // @listing resolve-field
    /// Resolves the `Fieldref` at `index` in `from`'s constant pool to the
    /// class that declares the field, and the field (JVMS 5.4.3.2).
    pub fn resolve_field(
        &mut self,
        from: &Class,
        index: u16,
    ) -> Result<(Rc<Class>, Field)> {
        let target = from.file.constant_pool.member(index)?;
        let class = self.resolve_class(from, target.class)?;
        let Some((owner, field)) = find_field(&class, target.name, target.descriptor)
        else {
            let ty = FieldType::parse(target.descriptor)?;
            let m = format!(
                "Class {} does not have member field '{ty} {}'",
                dotted(target.class),
                target.name
            );
            return Err(linkage("java/lang/NoSuchFieldError", m));
        };
        let what = format!("field {}.{}", dotted(&owner.file.this_class), field.name);
        check_access(from, &owner, field.access_flags, &what)?;
        Ok((owner, field))
    }
    // @end

    // @listing resolve-method
    /// Resolves the `Methodref` at `index` in `from`'s constant pool to the
    /// class that declares the method, and its index there (JVMS 5.4.3.3).
    pub fn resolve_method(
        &mut self,
        from: &Class,
        index: u16,
    ) -> Result<(Rc<Class>, usize)> {
        // @hide
        if let Some(Resolved::Method(class, method)) = from.cached(index) {
            return Ok((class.clone(), *method));
        }
        let constant = from.file.constant_pool.get(index)?;
        if let Constant::InterfaceMethodref { .. } = constant {
            return self.resolve_interface_method(from, index);
        }
        // @show
        let target = from.file.constant_pool.member(index)?;
        let class = self.resolve_class(from, target.class)?;
        let ty = MethodDescriptor::parse(target.descriptor)?;
        if class.file.access_flags.has(AccessFlags::INTERFACE) {
            let m = format!(
                "Found interface {}, but class was expected",
                dotted(target.class)
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        let Some((owner, method)) =
            find_method(&class, target.name, target.descriptor)
        else {
            let m = format!("'{}'", external(target.class, target.name, &ty));
            return Err(linkage("java/lang/NoSuchMethodError", m));
        };
        let flags = owner.file.methods[method].access_flags;
        let what = format!(
            "method '{}'",
            external(&owner.file.this_class, target.name, &ty)
        );
        check_access(from, &owner, flags, &what)?;
        // @hide
        from.cache(index, Resolved::Method(owner.clone(), method));
        // @show
        Ok((owner, method))
    }
    // @end
}

// @listing find
/// The field as JVMS 5.4.3.2 looks it up: in the class, then its
/// superinterfaces, then its superclass, each recursively.
fn find_field(
    class: &Rc<Class>,
    name: &str,
    descriptor: &str,
) -> Option<(Rc<Class>, Field)> {
    let fields = &class.file.fields;
    if let Some(f) =
        fields.iter().find(|f| f.name == name && f.descriptor == descriptor)
    {
        return Some((class.clone(), f.clone()));
    }
    let interfaces = class.interfaces.iter();
    let found = interfaces.filter_map(|i| find_field(i, name, descriptor)).next();
    found.or_else(|| find_field(class.superclass.as_ref()?, name, descriptor))
}

/// The method as JVMS 5.4.3.3 looks it up: in the class and its
/// superclasses, then in the superinterfaces of each of them.
fn find_method(
    class: &Rc<Class>,
    name: &str,
    descriptor: &str,
) -> Option<(Rc<Class>, usize)> {
    let mut c = Some(class.clone());
    while let Some(current) = c {
        if let Ok(index) = current.method(name, descriptor) {
            return Some((current, index));
        }
        c = current.superclass.clone();
    }
    let mut c = Some(class.clone());
    while let Some(current) = c {
        let interfaces = current.interfaces.iter();
        let found =
            interfaces.filter_map(|i| find_method(i, name, descriptor)).next();
        if found.is_some() {
            return found;
        }
        c = current.superclass.clone();
    }
    None
}
// @end

// @listing check-access
/// JVMS 5.4.4: may code in `from` use a member of `owner` with `flags`?
/// `what` names the member for the error: `field Rectangle.width`.
fn check_access(
    from: &Class,
    owner: &Rc<Class>,
    flags: AccessFlags,
    what: &str,
) -> Result<()> {
    let (from_name, owner_name) = (&from.file.this_class, &owner.file.this_class);
    let same_package = package(from_name) == package(owner_name);
    let allowed = if flags.has(AccessFlags::PUBLIC) {
        true
    } else if flags.has(AccessFlags::PRIVATE) {
        nest_host(from) == nest_host(owner) && same_package
    } else if flags.has(AccessFlags::PROTECTED) {
        same_package || is_subclass(from, owner)
    } else {
        same_package
    };
    if allowed {
        return Ok(());
    }
    // Only a method can be abstract; HotSpot names that first.
    let abstract_ = if flags.has(AccessFlags::ABSTRACT) { "abstract " } else { "" };
    let visibility = if flags.has(AccessFlags::PRIVATE) {
        "private "
    } else if flags.has(AccessFlags::PROTECTED) {
        "protected "
    } else {
        ""
    };
    let m = format!(
        "class {} tried to access {abstract_}{visibility}{what}",
        dotted(from_name)
    );
    Err(linkage("java/lang/IllegalAccessError", m))
}

/// The host of `class`'s nest: the class its `NestHost` attribute names, or
/// the class itself (JVMS 5.4.4).
pub(crate) fn nest_host(class: &Class) -> String {
    let attribute = class.file.attributes.iter().find(|a| a.name == "NestHost");
    let host = attribute.and_then(|a| {
        let index = Reader::new(&a.info).u2().ok()?;
        class.file.constant_pool.class_name(index).ok()
    });
    host.unwrap_or(&class.file.this_class).to_string()
}

/// Whether `class` is `ancestor` or one of its subclasses.
pub(crate) fn is_subclass(class: &Class, ancestor: &Rc<Class>) -> bool {
    if class.file.this_class == ancestor.file.this_class {
        return true;
    }
    class.superclass.as_ref().is_some_and(|s| is_subclass(s, ancestor))
}
// @end

// @listing names
/// The package of a class name in internal form: `java/lang` for
/// `java/lang/Object`, and "" for a class in no package.
pub(crate) fn package(name: &str) -> &str {
    name.rfind('/').map_or("", |at| &name[..at])
}

/// A class name as Java writes it: `java.lang.Object`.
pub(crate) fn dotted(name: &str) -> String {
    name.replace('/', ".")
}

/// A method as HotSpot names it in messages: `int Geometry.area(Rectangle)`.
pub(crate) fn external(class: &str, name: &str, ty: &MethodDescriptor) -> String {
    let ret = ty.ret.as_ref().map_or("void".to_string(), |t| t.to_string());
    let params: Vec<String> = ty.params.iter().map(|p| p.to_string()).collect();
    format!("{ret} {}.{name}({})", dotted(class), params.join(", "))
}
// @end

// @listing find-in-superclasses
/// The first method named `name` with `descriptor` declared in `class` or
/// its superclasses: the method `super.name()` calls (JVMS 6.5.invokespecial).
pub(crate) fn find_in_superclasses(
    class: &Rc<Class>,
    name: &str,
    descriptor: &str,
) -> Option<(Rc<Class>, usize)> {
    let mut c = Some(class.clone());
    while let Some(current) = c {
        if let Ok(index) = current.method(name, descriptor) {
            return Some((current, index));
        }
        c = current.superclass.clone();
    }
    None
}
// @end

impl Vm {
    // @listing resolve-interface-method
    /// Resolves the `InterfaceMethodref` at `index` in `from`'s pool (JVMS
    /// 5.4.3.4): a method of the interface, else a public method of `Object`,
    /// else a maximally specific method of its superinterfaces.
    pub fn resolve_interface_method(
        &mut self,
        from: &Class,
        index: u16,
    ) -> Result<(Rc<Class>, usize)> {
        let target = from.file.constant_pool.member(index)?;
        let class = self.resolve_class(from, target.class)?;
        if !class.file.access_flags.has(AccessFlags::INTERFACE) {
            let m = format!(
                "Found class {}, but interface was expected",
                dotted(target.class)
            );
            return Err(linkage("java/lang/IncompatibleClassChangeError", m));
        }
        let (name, descriptor) = (target.name, target.descriptor);
        let ty = MethodDescriptor::parse(descriptor)?;
        let object = class.superclass.clone().filter(|object| {
            object.method(name, descriptor).is_ok_and(|m| {
                let flags = object.file.methods[m].access_flags;
                flags.has(AccessFlags::PUBLIC) && !flags.has(AccessFlags::STATIC)
            })
        });
        let found = match class.method(name, descriptor) {
            Ok(m) => Some((class.clone(), m)),
            Err(_) => match object {
                Some(object) => {
                    Some((object.clone(), object.method(name, descriptor)?))
                }
                None => {
                    let specific = maximally_specific(&class, name, descriptor);
                    let concrete = specific.iter().find(|(c, m)| {
                        !c.file.methods[*m].access_flags.has(AccessFlags::ABSTRACT)
                    });
                    concrete.or(specific.first()).cloned()
                }
            },
        };
        let Some((owner, method)) = found else {
            let m = format!("'{}'", external(target.class, name, &ty));
            return Err(linkage("java/lang/NoSuchMethodError", m));
        };
        let flags = owner.file.methods[method].access_flags;
        let what =
            format!("method '{}'", external(&owner.file.this_class, name, &ty));
        check_access(from, &owner, flags, &what)?;
        // @hide
        from.cache(index, Resolved::Method(owner.clone(), method));
        // @show
        Ok((owner, method))
    }
    // @end
}

// @listing maximally-specific
/// The maximally specific superinterface methods of `class` with this name
/// and descriptor (JVMS 5.4.3.3): the non-private, non-static methods its
/// superinterfaces declare, without those that a more specific one, in a
/// subinterface, overrides.
pub(crate) fn maximally_specific(
    class: &Rc<Class>,
    name: &str,
    descriptor: &str,
) -> Vec<(Rc<Class>, usize)> {
    let mut interfaces: Vec<Rc<Class>> = Vec::new();
    superinterfaces(class, &mut interfaces);
    let skip = AccessFlags::PRIVATE | AccessFlags::STATIC;
    let candidates: Vec<(Rc<Class>, usize)> = interfaces
        .iter()
        .filter_map(|i| {
            let m = i.method(name, descriptor).ok()?;
            (!i.file.methods[m].access_flags.has(skip)).then(|| (i.clone(), m))
        })
        .collect();
    let overridden = |c: &Rc<Class>| {
        candidates.iter().any(|(d, _)| !Rc::ptr_eq(d, c) && is_subtype(d, c))
    };
    candidates.iter().filter(|(c, _)| !overridden(c)).cloned().collect()
}

/// Every superinterface of `class`, direct or not, through its superclasses
/// too, each once.
fn superinterfaces(class: &Rc<Class>, out: &mut Vec<Rc<Class>>) {
    for interface in &class.interfaces {
        if !out.iter().any(|seen| Rc::ptr_eq(seen, interface)) {
            out.push(interface.clone());
            superinterfaces(interface, out);
        }
    }
    if let Some(superclass) = &class.superclass {
        superinterfaces(superclass, out);
    }
}
// @end
