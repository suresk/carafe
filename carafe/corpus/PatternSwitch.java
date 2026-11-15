public class PatternSwitch {
    sealed interface Shape permits Circle, Square, Rect {}
    record Circle(double r) implements Shape {}
    record Square(double side) implements Shape {}
    record Rect(double w, double h) implements Shape {}

    static double area(Shape s) {
        return switch (s) {
            case Circle c -> 3 * c.r() * c.r();
            case Square q -> q.side() * q.side();
            case Rect(double w, double h) -> w * h;
        };
    }

    static String kind(Object o) {
        return switch (o) {
            case Integer i when i > 10 -> "big int";
            case Integer i -> "int";
            case String s -> "string of " + s.length();
            default -> "other";
        };
    }

    public static void main(String[] args) {
        System.out.println(area(new Circle(1)) + area(new Square(2)) + area(new Rect(2, 3)));
        System.out.println(kind(5) + ", " + kind(50) + ", " + kind("hey") + ", " + kind(1.5));
    }
}
