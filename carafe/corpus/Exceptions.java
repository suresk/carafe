public class Exceptions {
    static final class Res implements AutoCloseable {
        private final String name;
        Res(String name) { this.name = name; }
        public void close() { System.out.println("close " + name); }
    }

    static int divide(int a, int b) { return a / b; }

    public static void main(String[] args) {
        try { divide(1, 0); } catch (ArithmeticException e) { System.out.println("arith: " + e.getMessage()); }
        try { int[] a = new int[2]; a[5] = 1; } catch (ArrayIndexOutOfBoundsException e) { System.out.println("aioobe: " + e.getMessage()); }
        try { Object o = "s"; Integer i = (Integer) o; } catch (ClassCastException e) { System.out.println("cce"); }
        try { String n = null; n.length(); } catch (NullPointerException e) { System.out.println("npe"); }
        try { throw new IllegalStateException("bad"); }
        catch (RuntimeException e) { System.out.println(e); }
        finally { System.out.println("finally"); }
        try (Res a = new Res("a"); Res b = new Res("b")) { System.out.println("body"); }
        try { Integer.parseInt("x1"); } catch (NumberFormatException e) { System.out.println(e.getMessage()); }
    }
}
