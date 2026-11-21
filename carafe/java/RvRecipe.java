/// Two concatenations whose recipes the test patches so that they no longer
/// fit their descriptors: `one` gets an argument too few, `two` a constant
/// that isn't passed.
public class RvRecipe {
    static String one(int x, int y) {
        return "<" + x + "|" + y + ">";
    }

    static String two(int x, int y) {
        return "[" + x + "|" + y + "]";
    }

    public static void main(String[] a) {
        try {
            one(1, 2);
        } catch (BootstrapMethodError e) {
            System.out.println(e);
            System.out.println(e.getCause());
        }
        try {
            two(1, 2);
        } catch (BootstrapMethodError e) {
            System.out.println(e);
            System.out.println(e.getCause());
        }
    }
}
