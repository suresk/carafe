import java.lang.reflect.Method;

/// `Method.invoke` with a receiver of the wrong class, and with arguments
/// that must be widened, or can't be.
public class RvReflect {
    int id(int x) {
        return x;
    }

    static long twice(long x) {
        return 2 * x;
    }

    static double half(float x) {
        return x / 2;
    }

    static int narrow(int x) {
        return x;
    }

    public static void main(String[] a) throws Exception {
        Method id = RvReflect.class.getDeclaredMethod("id", int.class);
        try {
            id.invoke("not me", 1);
        } catch (IllegalArgumentException e) {
            System.out.println(e);
        }
        System.out.println(id.invoke(new RvReflect(), (short) 3));
        Method twice = RvReflect.class.getDeclaredMethod("twice", long.class);
        System.out.println(twice.invoke(null, 21));
        System.out.println(twice.invoke(null, 'a'));
        Method half = RvReflect.class.getDeclaredMethod("half", float.class);
        System.out.println(half.invoke(null, 5L));
        Method narrow = RvReflect.class.getDeclaredMethod("narrow", int.class);
        for (Object arg : new Object[] {5L, true, "5", null}) {
            try {
                narrow.invoke(null, arg);
            } catch (IllegalArgumentException e) {
                System.out.println(e);
            }
        }
    }
}
