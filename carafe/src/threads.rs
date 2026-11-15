//! The main thread, and standard input.

use crate::class::Class;
use crate::descriptor::FieldType;
use crate::error::Result;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::value::Value;
use std::rc::Rc;

// @listing current-thread
/// `Thread.currentThread()`: the main thread, the only one carafe has, made
/// the first time something asks for it.
pub(crate) fn current_thread(vm: &mut Vm, _: &[Value]) -> Result<Option<Value>> {
    let thread = match vm.thread {
        Some(thread) => thread,
        None => vm.main_thread()?,
    };
    Ok(Some(Value::Ref(thread)))
}
// @end

impl Vm {
    // @listing main-thread
    /// Makes the main thread as HotSpot does: a thread group called `system`,
    /// one called `main` inside it, and a `Thread` called `main` in that,
    /// made current before its constructor runs, since the constructor asks
    /// for the current thread to inherit from. A thread that is its own
    /// parent is one the JVM attached, which the constructor knows.
    fn main_thread(&mut self) -> Result<Ref> {
        let groups = self.load("java/lang/ThreadGroup")?;
        self.initialize(&groups)?;
        let system = self.construct_with(&groups, "()V", &[])?;
        self.heap.roots.push(system);
        let name = self.intern(&"main".encode_utf16().collect::<Vec<u16>>())?;
        self.heap.roots.push(name);
        let parent = [Value::Ref(system), Value::Ref(name)];
        let group = self.construct_with(&groups, GROUP_INIT, &parent)?;
        self.heap.roots.push(group);
        let threads = self.load("java/lang/Thread")?;
        self.initialize(&threads)?;
        let thread = self.heap.new_object(&threads);
        self.thread = Some(thread);
        let init = threads.method("<init>", GROUP_INIT)?;
        let args = [Value::Ref(thread), Value::Ref(group), Value::Ref(name)];
        let made = self.invoke_method(&threads, init, &args);
        self.heap.roots.truncate(self.heap.roots.len() - 3);
        made?;
        // HotSpot then marks the thread running: alive and runnable in its
        // status, and with a native thread in `eetop`, which `isAlive` tests.
        let holder_type = "Ljava/lang/Thread$FieldHolder;";
        let Value::Ref(holder) = self.get(&threads, thread, "holder", holder_type)?
        else {
            unreachable!("a reference field")
        };
        let holders = self.load("java/lang/Thread$FieldHolder")?;
        self.put(&holders, holder, "threadStatus", "I", Value::Int(RUNNABLE))?;
        self.put(&threads, thread, "eetop", "J", Value::Long(1))?;
        Ok(thread)
    }

    /// The value of the instance field `name` of `object`.
    fn get(
        &mut self,
        class: &Class,
        object: Ref,
        name: &str,
        ty: &str,
    ) -> Result<Value> {
        let slot = class.field(name, ty)?;
        Ok(self.heap.read(object.0 + slot.offset, &slot.ty))
    }

    /// A new object of `class`, constructed with the `<init>` of this
    /// descriptor and `args`, which are kept in the roots meanwhile.
    fn construct_with(
        &mut self,
        class: &Rc<Class>,
        descriptor: &str,
        args: &[Value],
    ) -> Result<Ref> {
        let object = self.heap.new_object(class);
        self.heap.roots.push(object);
        let mut all = vec![Value::Ref(object)];
        all.extend_from_slice(args);
        let init = class.method("<init>", descriptor)?;
        let made = self.invoke_method(class, init, &all);
        let object = self.heap.roots.pop().expect("pushed above");
        made.map(|_| object)
    }
    // @end
}

/// HotSpot's `JavaThreadStatus::RUNNABLE`: JVMTI's alive and runnable bits.
const RUNNABLE: i32 = 0x0005;

/// `ThreadGroup(ThreadGroup parent, String name)`, and the same for `Thread`.
const GROUP_INIT: &str = "(Ljava/lang/ThreadGroup;Ljava/lang/String;)V";

// @listing read-stdin
/// `System.In.read0(byte[], int, int)`, in carafe's shim: copies the next
/// bytes of standard input into the array, up to `len` of them, and returns
/// how many, or -1 at the end. When the bytes that have arrived run out, it
/// reads more from the source, as many as one read gives, which from a
/// terminal is a line.
pub(crate) fn read_stdin(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Ref(array), Value::Int(off), Value::Int(len)] = *args else {
        unreachable!()
    };
    if vm.stdin_read >= vm.stdin.len() {
        let mut more = vec![0; len as usize];
        let count = match &mut vm.stdin_source {
            Some(source) => source.read(&mut more)?,
            None => 0,
        };
        vm.stdin.extend_from_slice(&more[..count]);
    }
    let left = &vm.stdin[vm.stdin_read.min(vm.stdin.len())..];
    if left.is_empty() {
        return Ok(Some(Value::Int(-1)));
    }
    let count = left.len().min(len as usize);
    let bytes = left[..count].to_vec();
    for (i, &b) in bytes.iter().enumerate() {
        let at = vm.heap.element(array, off + i as i32)?;
        vm.heap.write(at, &FieldType::Byte, Value::Int(i32::from(b as i8)))?;
    }
    vm.stdin_read += count;
    Ok(Some(Value::Int(count as i32)))
}
// @end
