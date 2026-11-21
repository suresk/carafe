import java.util.function.Function;

/// A lambda whose handle the test patches to a `REF_invokeSpecial` of
/// another class's method, which carafe refuses.
public class RvSpecial {
    public static void main(String[] a) {
        Function<String, Integer> length = String::length;
        System.out.println(length.apply("four"));
    }
}
