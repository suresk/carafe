import java.util.*;

public class Collections2 {
    public static void main(String[] args) {
        TreeMap<String, Integer> counts = new TreeMap<>();
        for (String w : "the cat sat on the mat the end".split(" ")) counts.merge(w, 1, Integer::sum);
        System.out.println(counts);

        List<String> words = new ArrayList<>(counts.keySet());
        Collections.sort(words, Comparator.comparing(String::length).thenComparing(Comparator.reverseOrder()));
        System.out.println(words);

        Deque<Integer> stack = new ArrayDeque<>();
        LinkedList<Integer> queue = new LinkedList<>();
        for (int i = 0; i < 5; i++) { stack.push(i); queue.add(i * i); }
        System.out.println(stack.pop() + " " + queue.removeFirst() + " " + queue);

        Iterator<Integer> it = queue.iterator();
        while (it.hasNext()) if (it.next() % 2 == 0) it.remove();
        System.out.println(queue);

        int[] nums = {5, 3, 9, 1};
        Arrays.sort(nums);
        System.out.println(Arrays.toString(nums) + " " + Arrays.binarySearch(nums, 9));
    }
}
