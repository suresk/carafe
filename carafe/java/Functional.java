import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.function.BinaryOperator;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.IntBinaryOperator;
import java.util.function.Supplier;
import java.util.function.ToIntFunction;
import java.util.function.ToLongFunction;

public class Functional {
    private final String greeting;

    Functional(String greeting) {
        this.greeting = greeting;
    }

    Function<String, String> greeter() {
        return name -> greeting + ", " + name;
    }

    static Runnable task() {
        return () -> System.out.println("run");
    }

    public static void main(String[] args) {
        IntBinaryOperator add = (a, b) -> a + b;
        System.out.println(add.applyAsInt(2, 3));

        int base = 10;
        Function<Integer, Integer> plusBase = x -> x + base;
        System.out.println(plusBase.apply(5));

        BinaryOperator<Integer> sum = Integer::sum;
        System.out.println(sum.apply(20, 22));

        ToIntFunction<String> length = String::length;
        ToLongFunction<String> longLength = String::length;
        Function<String, Integer> boxedLength = String::length;
        System.out.println(length.applyAsInt("four") + longLength.applyAsLong("seven") + boxedLength.apply("ab"));

        Consumer<String> print = System.out::println;
        print.accept("bound");

        Supplier<List<String>> make = ArrayList::new;
        List<String> words = make.get();
        words.add("pear");
        words.add("fig");
        words.add("banana");
        words.add("kiwi");
        words.add("apple");
        Function<List<String>, Integer> size = List::size;
        System.out.println(size.apply(words));

        words.sort(Comparator.comparing(String::length).thenComparing(Comparator.reverseOrder()));
        System.out.println(words);
        words.removeIf(w -> w.length() > 4);
        words.forEach(w -> System.out.println("- " + w));

        System.out.println(new Functional("Hello").greeter().apply("lambda"));
        System.out.println(task() == task());
        task().run();

        int[] numbers = {5, 3, 9, 1, 7};
        Arrays.sort(numbers);
        System.out.println(Arrays.toString(numbers));
        String[] names = {"carafe", "Mug", "bottle"};
        Arrays.sort(names, String.CASE_INSENSITIVE_ORDER);
        System.out.println(String.join(" ", names));
    }
}
