import java.lang.annotation.RetentionPolicy;
import java.util.List;

public class Kinds {
    sealed interface Shape permits Circle, Square, Group {
    }

    record Circle(double radius) implements Shape {
    }

    record Square(int side) implements Shape {
    }

    record Group(String name, List<Shape> members) implements Shape {
    }

    record Mixed(boolean flag, char letter, byte small, short medium, long big, float ratio,
                 double precise, int[] numbers, Object nothing) {
    }

    enum Color {
        RED, GREEN, BLUE;

        Color next() {
            return values()[(ordinal() + 1) % values().length];
        }
    }

    static String describe(Shape shape) {
        return switch (shape) {
            case Circle c when c.radius() > 10 -> "a big circle";
            case Circle c -> "a circle of radius " + c.radius();
            case Square(int side) -> "a square of side " + side;
            case Group(String name, List<Shape> members) when members.isEmpty() -> "empty " + name;
            case Group g -> g.name() + " of " + g.members().size();
        };
    }

    static String size(Integer n) {
        return switch (n) {
            case 0 -> "zero";
            case 1 -> "one";
            case Integer i when i > 100 -> "many";
            case Integer i -> "some";
        };
    }

    static String word(String s) {
        return switch (s) {
            case "carafe" -> "a jug";
            case String t when t.length() > 5 -> "long";
            case String t -> "short";
        };
    }

    static String warmth(Color c) {
        switch (c) {
            case RED:
                return "warm";
            case BLUE:
                return "cool";
            default:
                return "neutral";
        }
    }

    static String kept(RetentionPolicy policy) {
        switch (policy) {
            case SOURCE:
                return "discarded";
            case RUNTIME:
                return "kept";
            default:
                return "in the class file";
        }
    }

    public static void main(String[] args) {
        Circle unit = new Circle(1.0);
        System.out.println(unit);
        System.out.println(new Group("pair", List.of(unit, new Square(2))));
        System.out.println(unit.equals(new Circle(1.0)) + " " + unit.equals(new Square(1)));
        System.out.println(new Circle(Double.NaN).equals(new Circle(Double.NaN)));
        System.out.println(new Circle(0.0).equals(new Circle(-0.0)));
        System.out.println(unit.hashCode() + " " + new Square(7).hashCode());
        Mixed mixed = new Mixed(true, 'x', (byte) -3, (short) 300, 1L << 40, 0.25f, Math.E, null, null);
        System.out.println(mixed.hashCode());
        System.out.println(new Mixed(false, 'y', (byte) 0, (short) 0, -1, Float.NaN, Double.NaN, null, "s").hashCode());
        System.out.println(mixed);
        System.out.println(mixed.equals(new Mixed(true, 'x', (byte) -3, (short) 300, 1L << 40, 0.25f, Math.E, null, null)));

        for (Shape s : List.of(new Circle(20), unit, new Square(3), new Group("none", List.of()),
                new Group("set", List.of(unit)))) {
            System.out.println(describe(s));
        }
        for (Integer n : new Integer[] {0, 1, 500, 7}) {
            System.out.print(size(n) + " ");
        }
        System.out.println();
        for (String s : new String[] {"carafe", "espresso", "tea"}) {
            System.out.print(word(s) + " ");
        }
        System.out.println();
        for (Color c : Color.values()) {
            System.out.println(c + " " + c.ordinal() + " " + warmth(c) + " " + c.next().name());
        }
        System.out.println(Color.BLUE.compareTo(Color.RED));
        for (RetentionPolicy policy : RetentionPolicy.values()) {
            System.out.println(policy + " " + kept(policy));
        }
    }
}
