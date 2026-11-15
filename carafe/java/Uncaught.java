public class Uncaught {
    static class Lazy {
        static final int VALUE = compute();

        static int compute() {
            throw new IllegalArgumentException("no value");
        }
    }

    static int read() {
        return Lazy.VALUE;
    }

    public static void main(String[] args) {
        System.out.println("before");
        read();
    }
}
