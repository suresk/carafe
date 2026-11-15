//! Native methods: the parts of the class library written in Rust.

use crate::arrays;
use crate::error::Result;
use crate::exceptions;
use crate::frame::Frame;
use crate::heap::Ref;
use crate::interpreter::Vm;
use crate::lang;
use crate::loader::linkage;
use crate::mirror;
use crate::reflection;
use crate::resolve::external;
use crate::unsafe_access;
use crate::value::Value;

// @listing native-fn
/// A native method: the VM and the arguments, `this` first for an instance
/// method; it returns the method's result.
pub type Native = fn(&mut Vm, &[Value]) -> Result<Option<Value>>;

/// The native method `class.name` with this descriptor, if carafe has it.
fn find(class: &str, name: &str, descriptor: &str) -> Option<Native> {
    // @hide
    if class == "jdk/internal/misc/Unsafe"
        && let Some(native) = unsafe_access::find(name)
    {
        return Some(native);
    }
    // @show
    Some(match format!("{class}.{name}{descriptor}").as_str() {
        "java/io/PrintStream.write0(ILjava/lang/String;)V" => write0,
        // @hide
        "java/lang/System.arraycopy(Ljava/lang/Object;ILjava/lang/Object;II)V" => {
            arrays::arraycopy
        }
        "java/lang/Object.clone()Ljava/lang/Object;" => arrays::clone,
        "java/lang/Class.registerNatives()V" => mirror::register_natives,
        "java/lang/Class.desiredAssertionStatus0(Ljava/lang/Class;)Z" => {
            mirror::desired_assertion_status
        }
        "java/lang/Class.getPrimitiveClass(Ljava/lang/String;)Ljava/lang/Class;" => {
            lang::get_primitive_class
        }
        "java/lang/Float.floatToRawIntBits(F)I" => lang::float_to_bits,
        "java/lang/Float.intBitsToFloat(I)F" => lang::bits_to_float,
        "java/lang/Double.doubleToRawLongBits(D)J" => lang::double_to_bits,
        "java/lang/Double.longBitsToDouble(J)D" => lang::bits_to_double,
        "jdk/internal/misc/VM.initialize()V" => lang::nothing,
        "java/lang/Runtime.maxMemory()J" => lang::max_memory,
        "java/lang/String.intern()Ljava/lang/String;" => Vm::intern_native,
        "java/lang/Runtime.availableProcessors()I" => |_, _| Ok(Some(Value::Int(1))),
        "java/lang/Object.getClass()Ljava/lang/Class;" => mirror::get_class,
        "java/lang/Class.initClassName()Ljava/lang/String;" => {
            mirror::init_class_name
        }
        "java/lang/Object.hashCode()I" => mirror::identity_hash,
        "java/lang/Class.isInstance(Ljava/lang/Object;)Z" => mirror::is_instance,
        "java/lang/Class.isAssignableFrom(Ljava/lang/Class;)Z" => {
            mirror::is_assignable_from
        }
        "java/lang/Class.getSuperclass()Ljava/lang/Class;" => mirror::get_superclass,
        "java/lang/System.identityHashCode(Ljava/lang/Object;)I" => {
            mirror::identity_hash
        }
        "jdk/internal/misc/CDS.getCDSConfigStatus()I" => lang::zero,
        "jdk/internal/misc/CDS.initializeFromArchive(Ljava/lang/Class;)V" => {
            lang::nothing
        }
        "jdk/internal/misc/CDS.getRandomSeedForDumping()J" => {
            |_, _| Ok(Some(Value::Long(0)))
        }
        "java/lang/Throwable.fillInStackTrace(I)Ljava/lang/Throwable;" => {
            exceptions::fill_in_stack_trace
        }
        "java/lang/ClassLoader.registerNatives()V" => lang::nothing,
        "java/lang/Class.isHidden()Z" => reflection::is_hidden,
        "java/lang/Class.getEnclosingMethod0()[Ljava/lang/Object;" => {
            reflection::get_enclosing_method
        }
        "java/lang/Class.getDeclaringClass0()Ljava/lang/Class;" => {
            reflection::get_declaring_class
        }
        "java/lang/Class.getSimpleBinaryName0()Ljava/lang/String;" => {
            reflection::get_simple_binary_name
        }
        "jdk/internal/reflect/Reflection.getClassAccessFlags(Ljava/lang/Class;)I" => {
            reflection::get_class_access_flags
        }
        "java/lang/ref/Reference.get0()Ljava/lang/Object;" => lang::reference_get,
        "java/lang/ref/Reference.refersTo0(Ljava/lang/Object;)Z" => lang::refers_to,
        "java/lang/ref/Reference.clear0()V" => lang::reference_clear,
        "java/lang/Class.forName0(Ljava/lang/String;ZLjava/lang/ClassLoader;Ljava/lang/Class;)\
         Ljava/lang/Class;" => reflection::for_name,
        "java/lang/Class.getConstantPool()Ljdk/internal/reflect/ConstantPool;" => {
            reflection::get_constant_pool
        }
        "java/lang/Class.getDeclaredConstructors0(Z)[Ljava/lang/reflect/Constructor;" => {
            reflection::get_declared_constructors
        }
        "jdk/internal/reflect/DirectConstructorHandleAccessor$NativeAccessor.newInstance0\
         (Ljava/lang/reflect/Constructor;[Ljava/lang/Object;)Ljava/lang/Object;" => {
            reflection::new_instance0
        }
        "java/lang/Class.getDeclaredFields0(Z)[Ljava/lang/reflect/Field;" => {
            reflection::get_declared_fields
        }
        "java/lang/Class.getDeclaredMethods0(Z)[Ljava/lang/reflect/Method;" => {
            reflection::get_declared_methods
        }
        "jdk/internal/reflect/DirectMethodHandleAccessor$NativeAccessor.invoke0\
         (Ljava/lang/reflect/Method;Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;" => {
            reflection::invoke0
        }
        "jdk/internal/reflect/Reflection.getCallerClass()Ljava/lang/Class;" => {
            reflection::get_caller_class
        }
        "java/lang/System.nanoTime()J" => lang::nano_time,
        "java/lang/System.currentTimeMillis()J" => lang::current_time_millis,
        "java/lang/reflect/Array.newArray(Ljava/lang/Class;I)Ljava/lang/Object;" => {
            mirror::new_array
        }
        "java/lang/NullPointerException.getExtendedNPEMessage()Ljava/lang/String;" => {
            |_, _| Ok(Some(Value::Ref(Ref::NULL)))
        }
        // @show
        _ => return None,
    })
}
// @end

impl Vm {
    // @listing call-native
    /// Runs a native method in place of a frame of bytecode, and pushes its
    /// result onto the caller's operand stack.
    pub(crate) fn call_native(&mut self, frame: Frame) -> Result<()> {
        let class = &frame.class.file.this_class;
        let method = &frame.class.file.methods[frame.method];
        let Some(native) = find(class, &method.name, &method.descriptor) else {
            let m = format!("'{}'", external(class, &method.name, &method.ty));
            return Err(linkage("java/lang/UnsatisfiedLinkError", m));
        };
        self.natives.insert(format!("{class}.{}{}", method.name, method.descriptor));
        if let Some(result) = native(self, &frame.args())? {
            self.frames.last_mut().expect("a caller").push(result)?;
        }
        Ok(())
    }
    // @end
}

// @listing write0
/// `PrintStream.write0(int fd, String s)`, in carafe's `PrintStream`: writes
/// the string, as UTF-8, to standard output or standard error.
fn write0(vm: &mut Vm, args: &[Value]) -> Result<Option<Value>> {
    let [Value::Int(fd), Value::Ref(s)] = *args else {
        unreachable!("checked descriptor")
    };
    let text = vm.rust_string(s)?;
    match fd {
        2 => vm.stderr.extend_from_slice(text.as_bytes()),
        _ => vm.stdout.extend_from_slice(text.as_bytes()),
    }
    Ok(None)
}
// @end

// @listing implemented
/// Whether carafe has the native method `class.name` with `descriptor`.
pub fn implemented(class: &str, name: &str, descriptor: &str) -> bool {
    find(class, name, descriptor).is_some()
}
// @end
