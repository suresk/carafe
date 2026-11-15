interface Shape {
    double area();

    default String name() {
        return "shape";
    }
}

abstract class Polygon implements Shape {
    static final int MAX_SIDES = 12;
    protected final int sides;

    Polygon(int sides) {
        this.sides = sides;
    }

    abstract double perimeter();
}

final class Square extends Polygon {
    private final double side;

    Square(double side) {
        super(4);
        this.side = side;
    }

    public double area() {
        return side * side;
    }

    double perimeter() {
        return sides * side;
    }
}

class Circle implements Shape {
    private final double radius;

    Circle(double radius) {
        this.radius = radius;
    }

    public double area() {
        return Math.PI * radius * radius;
    }

    public String name() {
        return "circle";
    }
}

public class Shapes {
    static long calls;

    static native long ticks();

    static double scale(double x, long times, int[] counts) {
        calls++;
        return x * times + counts.length;
    }

    public static void main(String[] args) {
        Shape[] shapes = { new Square(2), new Circle(1) };
        double total = 0;
        for (Shape s : shapes) {
            total += s.area();
        }
        System.out.println(scale(total, 2, new int[3]));
    }
}
