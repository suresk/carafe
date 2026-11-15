import java.util.List;

public class Concat {
    static String describe(Object o) {
        return "<" + o + ">";
    }

    public static void main(String[] args) {
        int count = 3;
        long big = 1L << 40;
        double half = 0.5;
        float third = 1.0f / 3;
        char c = 'x';
        boolean yes = true;
        byte b = -7;
        short s = 300;
        String none = null;
        double four = 4.0;
        System.out.println("ok " + four);
        System.out.println("ok " + 4.0);
        System.out.println("count=" + count + ", big=" + big + ", half=" + half);
        System.out.println(third + " " + c + yes + b + s + none);
        System.out.println("a\u0001b" + count + "\u0002" + c);
        System.out.println(describe(List.of(1, 2)) + describe(new int[0].length));
        double[] doubles = {1e-5, 1e21, Double.MIN_VALUE, 100.0 / 3, -0.0, Double.NaN};
        float[] floats = {0.1f, (float) Math.PI, Float.MAX_VALUE, 1e-10f};
        System.out.println(doubles[0] + " " + doubles[1] + " " + doubles[2] + " " + doubles[3]);
        System.out.println(doubles[4] + "|" + doubles[5] + "|" + floats[0] + "|" + floats[1]);
        System.out.println(floats[2] + "|" + floats[3]);
        String total = "";
        for (int i = 0; i < 5; i++) {
            total += i;
        }
        System.out.println(total);
        long min = Long.MIN_VALUE;
        System.out.println("" + min + Integer.MIN_VALUE + Character.MAX_VALUE);
    }
}
