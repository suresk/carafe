import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class Bench {
    interface Shape {
        double area();
    }

    record Square(double side) implements Shape {
        public double area() {
            return side * side;
        }
    }

    record Circle(double radius) implements Shape {
        public double area() {
            return 3.14159 * radius * radius;
        }
    }

    static final class Point {
        int x;
        int y;

        Point(int x, int y) {
            this.x = x;
            this.y = y;
        }
    }

    static int fib(int n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }

    static long fields(int n) {
        Point p = new Point(0, 0);
        long sum = 0;
        for (int i = 0; i < n; i++) {
            p.x += i;
            p.y ^= p.x;
            sum += p.y;
        }
        return sum;
    }

    static double shapes(int n) {
        Shape[] shapes = {new Square(2), new Circle(1), new Square(3)};
        double total = 0;
        for (int i = 0; i < n; i++) {
            total += shapes[i % 3].area();
        }
        return total;
    }

    static int sieve(int n) {
        boolean[] composite = new boolean[n + 1];
        int count = 0;
        for (int i = 2; i <= n; i++) {
            if (!composite[i]) {
                count++;
                for (int j = i * 2; j <= n; j += i) {
                    composite[j] = true;
                }
            }
        }
        return count;
    }

    static int collections(int n) {
        Map<Integer, List<Integer>> groups = new HashMap<>();
        for (int i = 0; i < n; i++) {
            groups.computeIfAbsent(i % 97, k -> new ArrayList<>()).add(i);
        }
        int total = 0;
        for (List<Integer> group : groups.values()) {
            total += group.size();
        }
        return total;
    }

    /**
     * Runs the benchmark named on standard input, with the size after its
     * name, or all of them at a small size if the input is empty.
     */
    public static void main(String[] args) throws java.io.IOException {
        String line = new java.io.BufferedReader(
                new java.io.InputStreamReader(System.in)).readLine();
        String[] words = line == null || line.isBlank() ? new String[0] : line.split(" ");
        String which = words.length > 0 ? words[0] : "all";
        int n = words.length > 1 ? Integer.parseInt(words[1]) : 0;
        boolean all = which.equals("all");
        if (all || which.equals("fib")) {
            System.out.println(fib(all ? 20 : n));
        }
        if (all || which.equals("fields")) {
            System.out.println(fields(all ? 100_000 : n));
        }
        if (all || which.equals("shapes")) {
            System.out.println(shapes(all ? 100_000 : n));
        }
        if (all || which.equals("sieve")) {
            System.out.println(sieve(all ? 100_000 : n));
        }
        if (all || which.equals("collections")) {
            System.out.println(collections(all ? 20_000 : n));
        }
    }
}
