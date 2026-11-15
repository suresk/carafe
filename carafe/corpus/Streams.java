import java.util.List;
import java.util.stream.Collectors;

public class Streams {
    public static void main(String[] args) {
        List<String> words = List.of("apple", "fig", "banana", "kiwi");
        System.out.println(words.stream().filter(w -> w.length() > 3).map(String::toUpperCase).collect(Collectors.toList()));
        System.out.println(words.stream().mapToInt(String::length).sum());
    }
}
