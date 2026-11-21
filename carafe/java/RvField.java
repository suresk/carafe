import java.lang.reflect.Field;

/// `Field.get` and `Field.set`, which need `java.lang.invoke`.
public class RvField {
    int x = 7;

    public static void main(String[] a) throws Exception {
        Field x = RvField.class.getDeclaredField("x");
        try {
            System.out.println(x.get(new RvField()));
        } catch (InternalError e) {
            System.out.println(e);
        }
        try {
            x.set(new RvField(), 8);
        } catch (InternalError e) {
            System.out.println(e);
        }
    }
}
