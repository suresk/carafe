import java.util.HashSet;
import java.util.Set;

public class Records {
    record Point(int x, int y) {}
    record Named(String name, Point at) {}

    public static void main(String[] args) {
        Point p = new Point(1, 2);
        System.out.println(p);
        System.out.println(p.equals(new Point(1, 2)));
        System.out.println(p.hashCode() == new Point(1, 2).hashCode());
        Set<Point> seen = new HashSet<>();
        seen.add(p); seen.add(new Point(1, 2)); seen.add(new Point(3, 4));
        System.out.println(seen.size());
        System.out.println(new Named("origin", new Point(0, 0)));
    }
}
