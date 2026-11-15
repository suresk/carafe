import java.util.ArrayList;
import java.util.List;

public class Library {
    public static void main(String[] args) {
        List<Integer> squares = new ArrayList<>();
        for (int i = 0; i < 20; i++) {
            squares.add(i * i);
        }
        System.out.println(squares.size());
        System.out.println(squares.get(19));
        StringBuilder builder = new StringBuilder();
        builder.append("x = ").append(42).append(", y = ").append(-1L).append('!');
        System.out.println(builder.toString());
        System.out.println(String.valueOf(new char[] {'h', 'é'}).length());
        System.out.println("Hello".toUpperCase());
    }
}
