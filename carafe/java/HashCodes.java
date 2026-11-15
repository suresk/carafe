public class HashCodes {
    // Runs only on HotSpot: the hash codes of a few strings.
    public static void main(String[] args) {
        String[] samples = {"", "a", "Hello, world!", "café", "a€", "😀", "polygenelubricants"};
        for (String s : samples) {
            System.out.println(s.hashCode());
        }
    }
}
