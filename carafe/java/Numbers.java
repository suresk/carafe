public class Numbers {
    static long longs(long a, long b, int op) {
        switch (op) {
            case 0: return a + b;
            case 1: return a - b;
            case 2: return a * b;
            case 3: return a / b;
            case 4: return a % b;
            case 5: return a << b;
            case 6: return a >> b;
            case 7: return a >>> b;
            case 8: return a & b;
            case 9: return a | b;
            case 10: return a ^ b;
            default: return -a;
        }
    }

    static float floats(float a, float b, int op) {
        switch (op) {
            case 0: return a + b;
            case 1: return a - b;
            case 2: return a * b;
            case 3: return a / b;
            case 4: return a % b;
            default: return -a;
        }
    }

    static double doubles(double a, double b, int op) {
        switch (op) {
            case 0: return a + b;
            case 1: return a - b;
            case 2: return a * b;
            case 3: return a / b;
            case 4: return a % b;
            default: return -a;
        }
    }

    // One bit per comparison: <, >, ==, <=, >=, !=.
    static int compareLongs(long a, long b) {
        return (a < b ? 1 : 0) | (a > b ? 2 : 0) | (a == b ? 4 : 0)
            | (a <= b ? 8 : 0) | (a >= b ? 16 : 0) | (a != b ? 32 : 0);
    }

    static int compareFloats(float a, float b) {
        return (a < b ? 1 : 0) | (a > b ? 2 : 0) | (a == b ? 4 : 0)
            | (a <= b ? 8 : 0) | (a >= b ? 16 : 0) | (a != b ? 32 : 0);
    }

    static int compareDoubles(double a, double b) {
        return (a < b ? 1 : 0) | (a > b ? 2 : 0) | (a == b ? 4 : 0)
            | (a <= b ? 8 : 0) | (a >= b ? 16 : 0) | (a != b ? 32 : 0);
    }

    static long i2l(int x) { return x; }
    static float i2f(int x) { return x; }
    static double i2d(int x) { return x; }
    static byte i2b(int x) { return (byte) x; }
    static char i2c(int x) { return (char) x; }
    static short i2s(int x) { return (short) x; }
    static int l2i(long x) { return (int) x; }
    static float l2f(long x) { return x; }
    static double l2d(long x) { return x; }
    static int f2i(float x) { return (int) x; }
    static long f2l(float x) { return (long) x; }
    static double f2d(float x) { return x; }
    static int d2i(double x) { return (int) x; }
    static long d2l(double x) { return (long) x; }
    static float d2f(double x) { return (float) x; }

    // The rest runs only on HotSpot: it prints each call and its result,
    // with float and double values as their raw bits.

    static final int[] INTS = {0, -1, 300, 65535, 70000, 16777217, Integer.MIN_VALUE,
        Integer.MAX_VALUE};
    static final long[] LONGS = {0, -1, 5, -7, 65, 1L << 40, 9007199254740993L,
        Long.MIN_VALUE, Long.MAX_VALUE};
    static final float[] FLOATS = {0f, -0f, 1.5f, -2.5f, 5.5f, 1e10f, -1e10f, 3e38f,
        1.4e-45f, Float.NaN, Float.POSITIVE_INFINITY, Float.NEGATIVE_INFINITY};
    static final double[] DOUBLES = {0d, -0d, 0.1, -2.9, 5.5, 1e10, -1e300, 1e300,
        3.4028235677973366e38, 4.9e-324, Double.NaN, Double.POSITIVE_INFINITY,
        Double.NEGATIVE_INFINITY};

    static String i(int x) { return "I:" + x; }
    static String j(long x) { return "J:" + x; }
    static String f(float x) { return "F:" + Float.floatToRawIntBits(x); }
    static String d(double x) { return "D:" + Double.doubleToRawLongBits(x); }

    public static void main(String[] args) {
        for (long a : LONGS) {
            for (long b : LONGS) {
                for (int op = 0; op <= 11; op++) {
                    if ((op == 3 || op == 4) && b == 0) {
                        continue;
                    }
                    System.out.println("longs (JJI)J " + j(a) + " " + j(b) + " " + i(op)
                        + " = " + j(longs(a, b, op)));
                }
                System.out.println("compareLongs (JJ)I " + j(a) + " " + j(b) + " = "
                    + i(compareLongs(a, b)));
            }
            System.out.println("l2i (J)I " + j(a) + " = " + i(l2i(a)));
            System.out.println("l2f (J)F " + j(a) + " = " + f(l2f(a)));
            System.out.println("l2d (J)D " + j(a) + " = " + d(l2d(a)));
        }
        for (float a : FLOATS) {
            for (float b : FLOATS) {
                for (int op = 0; op <= 5; op++) {
                    System.out.println("floats (FFI)F " + f(a) + " " + f(b) + " " + i(op)
                        + " = " + f(floats(a, b, op)));
                }
                System.out.println("compareFloats (FF)I " + f(a) + " " + f(b) + " = "
                    + i(compareFloats(a, b)));
            }
            System.out.println("f2i (F)I " + f(a) + " = " + i(f2i(a)));
            System.out.println("f2l (F)J " + f(a) + " = " + j(f2l(a)));
            System.out.println("f2d (F)D " + f(a) + " = " + d(f2d(a)));
        }
        for (double a : DOUBLES) {
            for (double b : DOUBLES) {
                for (int op = 0; op <= 5; op++) {
                    System.out.println("doubles (DDI)D " + d(a) + " " + d(b) + " " + i(op)
                        + " = " + d(doubles(a, b, op)));
                }
                System.out.println("compareDoubles (DD)I " + d(a) + " " + d(b) + " = "
                    + i(compareDoubles(a, b)));
            }
            System.out.println("d2i (D)I " + d(a) + " = " + i(d2i(a)));
            System.out.println("d2l (D)J " + d(a) + " = " + j(d2l(a)));
            System.out.println("d2f (D)F " + d(a) + " = " + f(d2f(a)));
        }
        for (int a : INTS) {
            System.out.println("i2l (I)J " + i(a) + " = " + j(i2l(a)));
            System.out.println("i2f (I)F " + i(a) + " = " + f(i2f(a)));
            System.out.println("i2d (I)D " + i(a) + " = " + d(i2d(a)));
            System.out.println("i2b (I)B " + i(a) + " = " + i(i2b(a)));
            System.out.println("i2c (I)C " + i(a) + " = " + i(i2c(a)));
            System.out.println("i2s (I)S " + i(a) + " = " + i(i2s(a)));
        }
    }
}
