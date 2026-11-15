import java.io.BufferedReader;
import java.io.InputStreamReader;

public class ReadInput {
    public static void main(String[] args) throws Exception {
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        int total = 0;
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.length() > 0) total += Integer.parseInt(line);
        }
        System.out.println("total " + total);
    }
}
