public class RvCycle {
    public static void main(String[] a) {
        RuntimeException x = new RuntimeException("x");
        RuntimeException y = new RuntimeException("y", x);
        x.initCause(y);
        throw y;
    }
}
