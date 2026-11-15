public class Arith {
    static int sum(int n) {
        int total = 0;
        for (int i = 1; i <= n; i++) {
            total += i;
        }
        return total;
    }

    static int gcd(int a, int b) {
        while (b != 0) {
            int t = a % b;
            a = b;
            b = t;
        }
        return a;
    }

    static int collatz(int n) {
        int steps = 0;
        while (n != 1) {
            n = n % 2 == 0 ? n / 2 : 3 * n + 1;
            steps++;
        }
        return steps;
    }

    static int bits(int x) {
        int count = 0;
        while (x != 0) {
            count += x & 1;
            x >>>= 1;
        }
        return count;
    }

    static int next(int x) {
        return x + 1;
    }

    static int divide(int a, int b) {
        return a / b;
    }
}
