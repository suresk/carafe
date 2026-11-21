//! Chapter 16: Exceptions.

use crate::error::Result;
use crate::exceptions::TraceFrame;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::value::Value;

// @listing static-int
/// The value of the static `int` field `name` of the class `class`.
pub fn static_int(vm: &mut Vm, class: &str, name: &str) -> Result<i32> {
    let class = vm.load(class)?;
    let slot = class.static_slot(name, "I");
    let missing =
        || crate::error::Error::Unsupported(format!("no static int {name}"));
    match class.statics.borrow()[slot.ok_or_else(missing)?] {
        Value::Int(v) => Ok(v),
        _ => Err(missing()),
    }
}
// @end

// @listing full-report
/// `Vm::report` with suppressed exceptions, as `Throwable.printStackTrace`
/// prints them: each one indented by a tab more than the exception that
/// suppressed it, before that exception's cause. An exception met a second
/// time is a `[CIRCULAR REFERENCE: …]`.
pub fn full_report(vm: &mut Vm, exception: Ref) -> Result<String> {
    let mut out = String::from("Exception in thread \"main\" ");
    enclosed(vm, &mut out, exception, &[], ("", ""), &mut Vec::new())?;
    Ok(out)
}

fn enclosed(
    vm: &mut Vm,
    out: &mut String,
    exception: Ref,
    enclosing: &[TraceFrame],
    (caption, prefix): (&str, &str),
    printed: &mut Vec<Ref>,
) -> Result<()> {
    if printed.contains(&exception) {
        let this = vm.java_to_string(exception)?;
        *out += &format!("{prefix}{caption}[CIRCULAR REFERENCE: {this}]\n");
        return Ok(());
    }
    printed.push(exception);
    *out += &format!("{prefix}{caption}{}\n", vm.java_to_string(exception)?);
    let trace = vm.backtraces.get(&exception).cloned().unwrap_or_default();
    let shared = trace.iter().rev().zip(enclosing.iter().rev());
    let common = shared.take_while(|(a, b)| a == b).count();
    for frame in &trace[..trace.len() - common] {
        *out += &format!("{prefix}{}\n", frame.line());
    }
    if common > 0 {
        *out += &format!("{prefix}\t... {common} more\n");
    }
    let throwable = vm.load("java/lang/Throwable")?;
    let method = throwable.method("getSuppressed", "()[Ljava/lang/Throwable;")?;
    let Some(Value::Ref(array)) =
        vm.invoke_method(&throwable, method, &[Value::Ref(exception)])?
    else {
        unreachable!("getSuppressed returns an array")
    };
    let element = crate::descriptor::FieldType::Object("java/lang/Throwable".into());
    for i in 0..vm.heap.array_length(array) as i32 {
        let Value::Ref(suppressed) =
            vm.heap.read(vm.heap.element(array, i)?, &element)
        else {
            unreachable!("a Throwable[]")
        };
        let indented = format!("{prefix}\t");
        let caption = ("Suppressed: ", indented.as_str());
        enclosed(vm, out, suppressed, &trace, caption, printed)?;
    }
    let cause = vm.java_cause(exception)?;
    if cause != Ref::NULL {
        enclosed(vm, out, cause, &trace, ("Caused by: ", prefix), printed)?;
    }
    Ok(())
}
// @end
