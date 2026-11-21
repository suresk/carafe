public class RvPrint {
    public static void main(String[] a) {
        try {
            Integer.parseInt("x");
        } catch (NumberFormatException e) {
            System.out.println(e.getStackTrace().length > 0);
            System.out.println(e.getStackTrace()[0].getMethodName());
            e.printStackTrace();
        }
        RuntimeException x = new RuntimeException("x");
        RuntimeException y = new RuntimeException("y", x);
        x.initCause(y);
        y.addSuppressed(new IllegalStateException("s"));
        y.printStackTrace();
    }
}
