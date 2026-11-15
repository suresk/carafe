import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Deque;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.TreeMap;

public class Collections15 {
    public static void main(String[] args) {
        String[] words = {"the", "cat", "sat", "on", "the", "mat", "the", "end"};
        Map<String, Integer> counts = new HashMap<>();
        for (String word : words) {
            Integer count = counts.get(word);
            counts.put(word, count == null ? 1 : count + 1);
        }
        System.out.println(counts);
        TreeMap<String, Integer> sorted = new TreeMap<>(counts);
        System.out.println(sorted);
        System.out.println(sorted.firstKey());
        System.out.println(sorted.headMap("on"));

        Deque<Integer> stack = new ArrayDeque<>();
        LinkedList<Integer> queue = new LinkedList<>();
        for (int i = 0; i < 5; i++) {
            stack.push(i);
            queue.add(i * i);
        }
        System.out.println(stack.pop());
        System.out.println(queue.removeFirst());
        System.out.println(queue);
        Iterator<Integer> it = queue.iterator();
        while (it.hasNext()) {
            if (it.next() % 2 == 0) {
                it.remove();
            }
        }
        System.out.println(queue);

        Set<Character> letters = new HashSet<>();
        for (char c : "mississippi".toCharArray()) {
            letters.add(c);
        }
        System.out.println(letters);

        List<String> list = new ArrayList<>(counts.keySet());
        Collections.sort(list);
        Collections.reverse(list);
        System.out.println(list);

        System.out.println(counts.getClass().getName());
        System.out.println(new int[0].getClass().getName());
        System.out.println(String[].class.getName());
        System.out.println(int.class.getName());
        System.out.println(Map.Entry.class.getName());
        Object object = new Object();
        System.out.println(object.hashCode() == System.identityHashCode(object));
        System.out.println(Objects.hash(1, "a", 'c'));
        System.out.println(counts.equals(new HashMap<>(sorted)));
    }
}
