import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class Modest {
    interface Shape { double area(); }

    static final class Square implements Shape {
        private final double side;
        Square(double side) { this.side = side; }
        public double area() { return side * side; }
    }

    static final class Rect implements Shape {
        private final double w, h;
        Rect(double w, double h) { this.w = w; this.h = h; }
        public double area() { return w * h; }
    }

    static final class TooBig extends Exception {
        TooBig(String message) { super(message); }
    }

    static void check(Shape s) throws TooBig {
        if (s.area() > 10) throw new TooBig("area " + s.area());
    }

    public static void main(String[] args) {
        List<String> words = new ArrayList<>();
        words.add("apple"); words.add("fig"); words.add("banana"); words.add("kiwi");

        Map<String, Integer> lengths = new HashMap<>();
        for (String w : words) lengths.put(w, w.length());

        StringBuilder sb = new StringBuilder();
        for (String w : words) sb.append(w.charAt(0));
        System.out.println("initials " + sb);
        System.out.println("banana has " + lengths.get("banana") + " letters");

        words.removeIf(w -> w.length() > 4);
        System.out.println("short words " + words.size());

        Shape[] shapes = { new Square(2), new Rect(3, 4) };
        for (Shape s : shapes) {
            try {
                check(s);
                System.out.println("ok " + s.area());
            } catch (TooBig e) {
                System.out.println("caught " + e.getMessage());
            }
        }
    }
}
