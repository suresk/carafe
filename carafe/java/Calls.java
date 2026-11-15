public class Calls {
    static int fib(int n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }

    static int depth(int n) {
        return n == 0 ? 0 : 1 + depth(n - 1);
    }

    static int forever(int n) {
        return forever(n + 1);
    }

    static long pick(int a, long b, int c) {
        return b;
    }

    static long middle(long x) {
        return pick(1, x, 2);
    }

    static double second(double a, double b) {
        return b;
    }

    static byte low(int x) {
        return (byte) x;
    }

    static void nothing() {
    }

    static int abs(int x) {
        return Math.abs(x);
    }
}
