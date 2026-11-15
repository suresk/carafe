import java.util.Arrays;
import java.util.Comparator;
import java.util.IntSummaryStatistics;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.OptionalDouble;
import java.util.TreeMap;
import java.util.function.Function;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import java.util.stream.Stream;

public class Pipelines {
    record City(String name, String country, int population) {
    }

    public static void main(String[] args) {
        List<City> cities = List.of(
                new City("Lyon", "France", 522),
                new City("Paris", "France", 2133),
                new City("Porto", "Portugal", 232),
                new City("Lisbon", "Portugal", 545),
                new City("Turin", "Italy", 848),
                new City("Milan", "Italy", 1371),
                new City("Rome", "Italy", 2749));

        System.out.println(IntStream.rangeClosed(1, 100).sum());
        System.out.println(IntStream.range(0, 20).filter(i -> i % 3 == 0).boxed().toList());
        OptionalDouble average = cities.stream().mapToInt(City::population).average();
        System.out.println(average.getAsDouble());
        IntSummaryStatistics stats = cities.stream().mapToInt(City::population).summaryStatistics();
        // IntSummaryStatistics.toString uses String.format, which carafe can't run.
        System.out.println(stats.getMin() + " " + stats.getMax() + " " + stats.getSum());

        Map<String, List<String>> byCountry = cities.stream().collect(Collectors.groupingBy(
                City::country, TreeMap::new, Collectors.mapping(City::name, Collectors.toList())));
        System.out.println(byCountry);
        Map<Boolean, Long> big = cities.stream().collect(
                Collectors.partitioningBy(c -> c.population() > 1000, Collectors.counting()));
        System.out.println(big);
        System.out.println(cities.stream().map(City::name).sorted(Comparator.reverseOrder())
                .collect(Collectors.joining(", ", "[", "]")));
        Optional<City> largest = cities.stream().max(Comparator.comparingInt(City::population));
        System.out.println(largest.map(City::name).orElse("none"));
        Map<String, Integer> totals = cities.stream().collect(
                Collectors.toMap(City::country, City::population, Integer::sum, TreeMap::new));
        System.out.println(totals);

        System.out.println(Stream.iterate(1, x -> x * 2).limit(12).map(String::valueOf)
                .collect(Collectors.joining(" ")));
        System.out.println(Stream.of("a,b", "c", "d,e,f").flatMap(s -> Arrays.stream(s.split(",")))
                .skip(1).distinct().count());
        System.out.println(LongStream.rangeClosed(1, 20).reduce(1, (a, b) -> a * b));
        Map<String, Long> fruit = Stream.of("kiwi", "fig", "apple", "fig", "kiwi")
                .collect(Collectors.groupingBy(Function.identity(), TreeMap::new,
                        Collectors.counting()));
        System.out.println(fruit);
        int[] squares = IntStream.rangeClosed(1, 10).map(i -> i * i).toArray();
        System.out.println(Arrays.stream(squares).anyMatch(i -> i == 49) + " "
                + Arrays.stream(squares).allMatch(i -> i > 0) + " " + Arrays.toString(squares));
        System.out.println(String.join("|", "chars".chars().mapToObj(c -> String.valueOf((char) c)).toList()));
    }
}
