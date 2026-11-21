public class RvSameLine {
    static void f() { throw new IllegalStateException("inner"); }
    static void g() { try { f(); } catch (IllegalStateException e) { throw new RuntimeException("outer", e); } }
    public static void main(String[] a) {
        g();
    }
}
