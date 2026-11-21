import java.util.function.IntUnaryOperator;
import java.util.function.ToLongFunction;

/// `g` returns an `Object` that the lambda class must unbox to a `long`
/// through `Number`, since the instantiated type names no wrapper. The test
/// patches `h`'s handle to `REF_invokeSpecial`, as `javac` emitted it for
/// a private method before Java 11.
public class RvLambda {
    private int base = 1;

    static <T> T id(T t) {
        return t;
    }

    private int plus(int x) {
        return base + x;
    }

    public static void main(String[] a) {
        ToLongFunction<Long> f = RvLambda::id;
        System.out.println(f.applyAsLong(5L));
        ToLongFunction<Integer> g = RvLambda::id;
        System.out.println(g.applyAsLong(7));
        IntUnaryOperator h = new RvLambda()::plus;
        System.out.println(h.applyAsInt(41));
    }
}
