public class Format {
    public static void main(String[] args) {
        // printf would be a one-line shim over String.format; measure String.format itself.
        System.out.println(String.format("%d items, %s, %5.2f%%", 3, "ok", 12.345));
        System.out.println(String.format("[%-6s|%6s] %x %08d", "ab", "cd", 255, 42));
    }
}
