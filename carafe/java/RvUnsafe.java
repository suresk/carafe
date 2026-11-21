import jdk.internal.misc.Unsafe;

/// Unsafe accesses outside an object; compiled and run with
/// `--add-exports java.base/jdk.internal.misc=ALL-UNNAMED`.
public class RvUnsafe {
    public static void main(String[] a) {
        Unsafe u = Unsafe.getUnsafe();
        int[] x = new int[2];
        int[] next = {5};
        u.putInt(x, 16L + 4, 7);
        System.out.println(x[1]);
        try { u.putInt(x, 16L + 8, 7); } catch (InternalError e) { System.out.println(e.getMessage()); }
        try { u.getInt(x, 1L << 30); } catch (InternalError e) { System.out.println(e.getMessage()); }
        try { u.getLong(x, -8); } catch (InternalError e) { System.out.println(e.getMessage()); }
        System.out.println(next[0]);
    }
}
