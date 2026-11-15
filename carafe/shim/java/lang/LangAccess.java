package java.lang;

import jdk.internal.access.JavaLangAccess;
import jdk.internal.reflect.ConstantPool;

// @listing lang-access
/**
 * carafe's {@code JavaLangAccess}: the private parts of {@code java.lang} that
 * the rest of {@code java.base} reaches through {@code SharedSecrets}. The
 * JDK's {@code System} registers its own during startup, which carafe's shim
 * replaces. The methods carafe's programs need call {@code Class} and
 * {@code String}; the others say which method was called.
 */
final class LangAccess implements JavaLangAccess {
    @Override
    public <E extends Enum<E>> E[] getEnumConstantsShared(Class<E> type) {
        return type.getEnumConstantsShared();
    }

    @Override
    public ConstantPool getConstantPool(Class<?> type) {
        return type.getConstantPool();
    }

    @Override
    public String join(String prefix, String suffix, String delimiter,
                       String[] elements, int size) {
        return String.join(prefix, suffix, delimiter, elements, size);
    }

    @Override
    public int uncheckedDecodeASCII(byte[] src, int srcOff, char[] dst, int dstOff,
                                    int len) {
        return String.decodeASCII(src, srcOff, dst, dstOff, len);
    }
    // @hide

    @Override
    public java.util.List<java.lang.reflect.Method> getDeclaredPublicMethods(java.lang.Class<?> a0, java.lang.String a1, java.lang.Class<?>... a2) {
        throw new UnsupportedOperationException("JavaLangAccess.getDeclaredPublicMethods");
    }

    @Override
    public java.lang.reflect.Method findMethod(java.lang.Class<?> a0, boolean a1, java.lang.String a2, java.lang.Class<?>... a3) {
        throw new UnsupportedOperationException("JavaLangAccess.findMethod");
    }

    @Override
    public java.util.Map<java.lang.Class<? extends java.lang.annotation.Annotation>, java.lang.annotation.Annotation> getDeclaredAnnotationMap(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getDeclaredAnnotationMap");
    }

    @Override
    public byte[] getRawClassAnnotations(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getRawClassAnnotations");
    }

    @Override
    public byte[] getRawClassTypeAnnotations(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getRawClassTypeAnnotations");
    }

    @Override
    public boolean casAnnotationType(java.lang.Class<?> a0, sun.reflect.annotation.AnnotationType a1, sun.reflect.annotation.AnnotationType a2) {
        throw new UnsupportedOperationException("JavaLangAccess.casAnnotationType");
    }

    @Override
    public sun.reflect.annotation.AnnotationType getAnnotationType(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getAnnotationType");
    }

    @Override
    public byte[] getRawExecutableTypeAnnotations(java.lang.reflect.Executable a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getRawExecutableTypeAnnotations");
    }

    @Override
    public int classFileVersion(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.classFileVersion");
    }

    @Override
    public void blockedOn(sun.nio.ch.Interruptible a0) {
        throw new UnsupportedOperationException("JavaLangAccess.blockedOn");
    }

    @Override
    public void registerShutdownHook(int a0, boolean a1, java.lang.Runnable a2) {
        throw new UnsupportedOperationException("JavaLangAccess.registerShutdownHook");
    }

    @Override
    public void invokeFinalize(java.lang.Object a0) throws java.lang.Throwable {
        throw new UnsupportedOperationException("JavaLangAccess.invokeFinalize");
    }

    @Override
    public java.util.concurrent.ConcurrentHashMap<?, ?> createOrGetClassLoaderValueMap(java.lang.ClassLoader a0) {
        throw new UnsupportedOperationException("JavaLangAccess.createOrGetClassLoaderValueMap");
    }

    @Override
    public java.lang.Class<?> defineClass(java.lang.ClassLoader a0, java.lang.String a1, byte[] a2, java.security.ProtectionDomain a3, java.lang.String a4) {
        throw new UnsupportedOperationException("JavaLangAccess.defineClass");
    }

    @Override
    public java.lang.Class<?> defineClass(java.lang.ClassLoader a0, java.lang.Class<?> a1, java.lang.String a2, byte[] a3, java.security.ProtectionDomain a4, boolean a5, int a6, java.lang.Object a7) {
        throw new UnsupportedOperationException("JavaLangAccess.defineClass");
    }

    @Override
    public java.lang.Class<?> findBootstrapClassOrNull(java.lang.String a0) {
        throw new UnsupportedOperationException("JavaLangAccess.findBootstrapClassOrNull");
    }

    @Override
    public java.lang.Package definePackage(java.lang.ClassLoader a0, java.lang.String a1, java.lang.Module a2) {
        throw new UnsupportedOperationException("JavaLangAccess.definePackage");
    }

    @Override
    public java.lang.Module defineModule(java.lang.ClassLoader a0, java.lang.module.ModuleDescriptor a1, java.net.URI a2) {
        throw new UnsupportedOperationException("JavaLangAccess.defineModule");
    }

    @Override
    public java.lang.Module defineUnnamedModule(java.lang.ClassLoader a0) {
        throw new UnsupportedOperationException("JavaLangAccess.defineUnnamedModule");
    }

    @Override
    public void addReads(java.lang.Module a0, java.lang.Module a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addReads");
    }

    @Override
    public void addReadsAllUnnamed(java.lang.Module a0) {
        throw new UnsupportedOperationException("JavaLangAccess.addReadsAllUnnamed");
    }

    @Override
    public void addExports(java.lang.Module a0, java.lang.String a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addExports");
    }

    @Override
    public void addExports(java.lang.Module a0, java.lang.String a1, java.lang.Module a2) {
        throw new UnsupportedOperationException("JavaLangAccess.addExports");
    }

    @Override
    public void addExportsToAllUnnamed(java.lang.Module a0, java.lang.String a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addExportsToAllUnnamed");
    }

    @Override
    public void addOpens(java.lang.Module a0, java.lang.String a1, java.lang.Module a2) {
        throw new UnsupportedOperationException("JavaLangAccess.addOpens");
    }

    @Override
    public void addOpensToAllUnnamed(java.lang.Module a0, java.lang.String a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addOpensToAllUnnamed");
    }

    @Override
    public void addUses(java.lang.Module a0, java.lang.Class<?> a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addUses");
    }

    @Override
    public boolean isReflectivelyExported(java.lang.Module a0, java.lang.String a1, java.lang.Module a2) {
        throw new UnsupportedOperationException("JavaLangAccess.isReflectivelyExported");
    }

    @Override
    public boolean isReflectivelyOpened(java.lang.Module a0, java.lang.String a1, java.lang.Module a2) {
        throw new UnsupportedOperationException("JavaLangAccess.isReflectivelyOpened");
    }

    @Override
    public java.lang.Module addEnableNativeAccess(java.lang.Module a0) {
        throw new UnsupportedOperationException("JavaLangAccess.addEnableNativeAccess");
    }

    @Override
    public boolean addEnableNativeAccess(java.lang.ModuleLayer a0, java.lang.String a1) {
        throw new UnsupportedOperationException("JavaLangAccess.addEnableNativeAccess");
    }

    @Override
    public void addEnableNativeAccessToAllUnnamed() {
        throw new UnsupportedOperationException("JavaLangAccess.addEnableNativeAccessToAllUnnamed");
    }

    @Override
    public void ensureNativeAccess(java.lang.Module a0, java.lang.Class<?> a1, java.lang.String a2, java.lang.Class<?> a3, boolean a4) {
        throw new UnsupportedOperationException("JavaLangAccess.ensureNativeAccess");
    }

    @Override
    public jdk.internal.module.ServicesCatalog getServicesCatalog(java.lang.ModuleLayer a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getServicesCatalog");
    }

    @Override
    public void bindToLoader(java.lang.ModuleLayer a0, java.lang.ClassLoader a1) {
        throw new UnsupportedOperationException("JavaLangAccess.bindToLoader");
    }

    @Override
    public java.util.stream.Stream<java.lang.ModuleLayer> layers(java.lang.ModuleLayer a0) {
        throw new UnsupportedOperationException("JavaLangAccess.layers");
    }

    @Override
    public java.util.stream.Stream<java.lang.ModuleLayer> layers(java.lang.ClassLoader a0) {
        throw new UnsupportedOperationException("JavaLangAccess.layers");
    }

    @Override
    public int uncheckedCountPositives(byte[] a0, int a1, int a2) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedCountPositives");
    }

    @Override
    public int countNonZeroAscii(java.lang.String a0) {
        throw new UnsupportedOperationException("JavaLangAccess.countNonZeroAscii");
    }

    @Override
    public java.lang.String uncheckedNewStringNoRepl(byte[] a0, java.nio.charset.Charset a1) throws java.nio.charset.CharacterCodingException {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedNewStringNoRepl");
    }

    @Override
    public byte[] uncheckedGetBytesNoRepl(java.lang.String a0, java.nio.charset.Charset a1) throws java.nio.charset.CharacterCodingException {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedGetBytesNoRepl");
    }

    @Override
    public java.lang.String newStringUTF8NoRepl(byte[] a0, int a1, int a2) {
        throw new UnsupportedOperationException("JavaLangAccess.newStringUTF8NoRepl");
    }

    @Override
    public char uncheckedGetUTF16Char(byte[] a0, int a1) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedGetUTF16Char");
    }

    @Override
    public void uncheckedPutCharUTF16(byte[] a0, int a1, int a2) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedPutCharUTF16");
    }

    @Override
    public byte[] getBytesUTF8NoRepl(java.lang.String a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getBytesUTF8NoRepl");
    }

    @Override
    public void uncheckedInflateBytesToChars(byte[] a0, int a1, char[] a2, int a3, int a4) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedInflateBytesToChars");
    }


    @Override
    public java.io.InputStream initialSystemIn() {
        throw new UnsupportedOperationException("JavaLangAccess.initialSystemIn");
    }

    @Override
    public java.io.PrintStream initialSystemErr() {
        throw new UnsupportedOperationException("JavaLangAccess.initialSystemErr");
    }

    @Override
    public int uncheckedEncodeASCII(char[] a0, int a1, byte[] a2, int a3, int a4) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedEncodeASCII");
    }

    @Override
    public void setCause(java.lang.Throwable a0, java.lang.Throwable a1) {
        throw new UnsupportedOperationException("JavaLangAccess.setCause");
    }

    @Override
    public java.security.ProtectionDomain protectionDomain(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.protectionDomain");
    }

    @Override
    public java.lang.invoke.MethodHandle stringConcatHelper(java.lang.String a0, java.lang.invoke.MethodType a1) {
        throw new UnsupportedOperationException("JavaLangAccess.stringConcatHelper");
    }

    @Override
    public long stringConcatInitialCoder() {
        throw new UnsupportedOperationException("JavaLangAccess.stringConcatInitialCoder");
    }

    @Override
    public long stringConcatMix(long a0, java.lang.String a1) {
        throw new UnsupportedOperationException("JavaLangAccess.stringConcatMix");
    }

    @Override
    public long stringConcatMix(long a0, char a1) {
        throw new UnsupportedOperationException("JavaLangAccess.stringConcatMix");
    }

    @Override
    public java.lang.Object uncheckedStringConcat1(java.lang.String[] a0) {
        throw new UnsupportedOperationException("JavaLangAccess.uncheckedStringConcat1");
    }

    @Override
    public byte stringInitCoder() {
        throw new UnsupportedOperationException("JavaLangAccess.stringInitCoder");
    }

    @Override
    public byte stringCoder(java.lang.String a0) {
        throw new UnsupportedOperationException("JavaLangAccess.stringCoder");
    }


    @Override
    public java.lang.String concat(java.lang.String a0, java.lang.Object a1, java.lang.String a2) {
        throw new UnsupportedOperationException("JavaLangAccess.concat");
    }

    @Override
    public java.lang.Object classData(java.lang.Class<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.classData");
    }

    @Override
    public jdk.internal.loader.NativeLibraries nativeLibrariesFor(java.lang.ClassLoader a0) {
        throw new UnsupportedOperationException("JavaLangAccess.nativeLibrariesFor");
    }

    @Override
    public java.lang.Thread[] getAllThreads() {
        throw new UnsupportedOperationException("JavaLangAccess.getAllThreads");
    }

    @Override
    public jdk.internal.vm.ThreadContainer threadContainer(java.lang.Thread a0) {
        throw new UnsupportedOperationException("JavaLangAccess.threadContainer");
    }

    @Override
    public void start(java.lang.Thread a0, jdk.internal.vm.ThreadContainer a1) {
        throw new UnsupportedOperationException("JavaLangAccess.start");
    }

    @Override
    public jdk.internal.vm.StackableScope headStackableScope(java.lang.Thread a0) {
        throw new UnsupportedOperationException("JavaLangAccess.headStackableScope");
    }

    @Override
    public void setHeadStackableScope(jdk.internal.vm.StackableScope a0) {
        throw new UnsupportedOperationException("JavaLangAccess.setHeadStackableScope");
    }

    @Override
    public java.lang.Thread currentCarrierThread() {
        throw new UnsupportedOperationException("JavaLangAccess.currentCarrierThread");
    }

    @Override
    public <T> T getCarrierThreadLocal(jdk.internal.misc.CarrierThreadLocal<T> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getCarrierThreadLocal");
    }

    @Override
    public <T> void setCarrierThreadLocal(jdk.internal.misc.CarrierThreadLocal<T> a0, T a1) {
        throw new UnsupportedOperationException("JavaLangAccess.setCarrierThreadLocal");
    }

    @Override
    public void removeCarrierThreadLocal(jdk.internal.misc.CarrierThreadLocal<?> a0) {
        throw new UnsupportedOperationException("JavaLangAccess.removeCarrierThreadLocal");
    }

    @Override
    public java.lang.Object[] scopedValueCache() {
        throw new UnsupportedOperationException("JavaLangAccess.scopedValueCache");
    }

    @Override
    public void setScopedValueCache(java.lang.Object[] a0) {
        throw new UnsupportedOperationException("JavaLangAccess.setScopedValueCache");
    }

    @Override
    public java.lang.Object scopedValueBindings() {
        throw new UnsupportedOperationException("JavaLangAccess.scopedValueBindings");
    }

    @Override
    public jdk.internal.vm.Continuation getContinuation(java.lang.Thread a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getContinuation");
    }

    @Override
    public void setContinuation(java.lang.Thread a0, jdk.internal.vm.Continuation a1) {
        throw new UnsupportedOperationException("JavaLangAccess.setContinuation");
    }

    @Override
    public jdk.internal.vm.ContinuationScope virtualThreadContinuationScope() {
        throw new UnsupportedOperationException("JavaLangAccess.virtualThreadContinuationScope");
    }

    @Override
    public void parkVirtualThread() {
        throw new UnsupportedOperationException("JavaLangAccess.parkVirtualThread");
    }

    @Override
    public void parkVirtualThread(long a0) {
        throw new UnsupportedOperationException("JavaLangAccess.parkVirtualThread");
    }

    @Override
    public void unparkVirtualThread(java.lang.Thread a0) {
        throw new UnsupportedOperationException("JavaLangAccess.unparkVirtualThread");
    }

    @Override
    public java.util.concurrent.Executor virtualThreadDefaultScheduler() {
        throw new UnsupportedOperationException("JavaLangAccess.virtualThreadDefaultScheduler");
    }

    @Override
    public java.lang.StackWalker newStackWalkerInstance(java.util.Set<java.lang.StackWalker.Option> a0, jdk.internal.vm.ContinuationScope a1, jdk.internal.vm.Continuation a2) {
        throw new UnsupportedOperationException("JavaLangAccess.newStackWalkerInstance");
    }

    @Override
    public java.lang.String getLoaderNameID(java.lang.ClassLoader a0) {
        throw new UnsupportedOperationException("JavaLangAccess.getLoaderNameID");
    }

    @Override
    public void copyToSegmentRaw(java.lang.String a0, java.lang.foreign.MemorySegment a1, long a2) {
        throw new UnsupportedOperationException("JavaLangAccess.copyToSegmentRaw");
    }

    @Override
    public boolean bytesCompatible(java.lang.String a0, java.nio.charset.Charset a1) {
        throw new UnsupportedOperationException("JavaLangAccess.bytesCompatible");
    }
    // @show
}
// @end
