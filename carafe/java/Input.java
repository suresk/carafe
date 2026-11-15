import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

public class Input {
    public static void main(String[] args) throws IOException {
        Thread main = Thread.currentThread();
        System.out.println(main.getName() + " in " + main.getThreadGroup().getName());
        System.out.println(main.getState() + " " + main.isAlive() + " " + main.getPriority()
                + " " + main.isDaemon());
        BufferedReader in = new BufferedReader(
                new InputStreamReader(System.in, StandardCharsets.UTF_8));
        long total = 0;
        int lines = 0;
        String line;
        while ((line = in.readLine()) != null) {
            lines++;
            for (String word : line.trim().split("\\s+")) {
                if (word.matches("-?\\d+")) {
                    total += Long.parseLong(word);
                } else if (!word.isEmpty()) {
                    System.out.println(word.toUpperCase() + " " + word.length());
                }
            }
        }
        System.out.println(lines + " lines, total " + total);
        System.out.println(System.in.read());
    }
}
