import java.util.*;
import java.util.function.*;
import java.util.stream.*;

public class Stress {
    record Pair(int a, String b) {}

    public static void main(String[] args) {
        Random random = new Random(42);
        List<Integer> numbers = random.ints(10_000, 0, 1000).boxed().toList();
        System.out.println(numbers.stream().sorted().skip(5000).findFirst().orElse(-1));
        System.out.println(numbers.stream().distinct().count());
        String range = numbers.stream().collect(Collectors.teeing(
            Collectors.minBy(Comparator.<Integer>naturalOrder()), Collectors.maxBy(Comparator.<Integer>naturalOrder()),
            (min, max) -> min.get() + ".." + max.get()));
        System.out.println(range);
        System.out.println(Stream.iterate(1, i -> i < 100, i -> i * 3).toList());
        System.out.println(IntStream.range(0, 30).boxed().takeWhile(i -> i < 5).dropWhile(i -> i < 2).toList());
        Supplier<Function<Integer, Function<Integer, Integer>>> adder = () -> a -> b -> a + b;
        System.out.println(adder.get().apply(3).apply(4));
        System.out.println(Stream.of(new Pair(1, "x"), new Pair(1, "x"), new Pair(2, "y")).distinct().toList());
        String text = """
            one two
            three
            """;
        System.out.println(text.lines().flatMap(l -> Arrays.stream(l.split(" "))).map(String::length).toList());
        System.out.println(Stream.of("a", "bb", "ccc").map(s -> switch (s.length()) {
            case 1 -> "one";
            case 2 -> { String t = s + s; yield t; }
            default -> "many";
        }).toList());
        BitSet bits = new BitSet();
        numbers.stream().limit(100).forEach(bits::set);
        System.out.println(bits.cardinality());
        Map<Integer, Long> histogram = numbers.stream().collect(
            Collectors.groupingBy(n -> n / 100, TreeMap::new, Collectors.counting()));
        System.out.println(histogram);
        System.out.println(new ArrayDeque<>(numbers.subList(0, 5)).reversed());
        System.out.println(Objects.hash(numbers.subList(0, 3).toArray()));
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 2000; i++) sb.append((char) ('a' + i % 26));
        System.out.println(sb.reverse().toString().hashCode());
        System.out.println(Long.toBinaryString(Long.MIN_VALUE >>> 3).length() + " " + Integer.toHexString(-255));
        System.out.println(Math.floorMod(-7, 3) + " " + Math.round(-2.5) + " " + Math.sqrt(2) + " " + Math.pow(2, 0.5));
        System.out.println(String.valueOf(new char[] {'o', 'k'}) + " " + "Straße".toUpperCase() + " " + "ǅ".toLowerCase());
    }
}
