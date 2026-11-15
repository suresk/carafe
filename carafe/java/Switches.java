public class Switches {
    static int days(int month) {
        switch (month) {
            case 2:
                return 28;
            case 4:
            case 6:
            case 9:
            case 11:
                return 30;
            default:
                return 31;
        }
    }

    static String size(int n) {
        switch (n) {
            case -5:
                return "negative";
            case 10:
                return "ten";
            case 1000:
                return "thousand";
            default:
                return "other";
        }
    }

    static long total(long[] values) {
        long sum = 0;
        for (int i = 0; i < values.length; i += 300) {
            sum += values[i] * 100_000L;
        }
        return sum;
    }

    static Object grid(Object o) {
        int[][] cells = new int[3][4];
        char[] letters = new char[2];
        if (o instanceof String s) {
            return s.length() > 3 ? cells : letters;
        }
        return o == null ? (Object) 2.5f : (Comparable<?>) o;
    }
}
