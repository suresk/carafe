package p;

public class Same {
    public static Object make() {
        return new Hidden[0];
    }

    public static int cast(Object o) {
        return ((Hidden[]) o).length;
    }
}
