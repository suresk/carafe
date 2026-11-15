import java.util.*;

public class Factories {
    public static void main(String[] args) {
        List<String> l = List.of("a", "b", "c");
        Map<String, Integer> m = Map.of("one", 1);
        Set<Integer> s = Set.of(7);
        System.out.println(l + " " + m.get("one") + " " + s.contains(7));
        List<Integer> u = Collections.unmodifiableList(new ArrayList<>(List.of(1, 2)));
        System.out.println(u.size());
    }
}
