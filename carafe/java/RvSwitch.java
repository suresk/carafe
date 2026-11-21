/// Pattern switches whose labels are dynamic constants: a qualified enum
/// constant, and the primitive type of a guarded record pattern's component.
public class RvSwitch {
    sealed interface S permits C, R {}

    enum C implements S { X, Y }

    record R(int side) implements S {}

    static int f(S s) {
        return switch (s) {
            case C.X -> 1;
            case C c -> 2;
            case R r -> 3;
        };
    }

    static int g(S s) {
        return switch (s) {
            case R(int side) when side > 3 -> 1;
            case R(int side) -> 2;
            case C c -> 3;
        };
    }

    public static void main(String[] a) {
        for (S s : new S[] {C.X, C.Y, new R(5), new R(2)}) {
            System.out.println(f(s));
            System.out.println(g(s));
        }
    }
}
