import java.util.function.IntBinaryOperator;

public class Lambdas {
    record Point(int x, int y) {}

    public static void main(String[] args) {
        IntBinaryOperator add = (a, b) -> a + b;
        Point p = new Point(1, 2);
        System.out.println("sum " + add.applyAsInt(p.x(), p.y()) + " of " + p);
    }
}
