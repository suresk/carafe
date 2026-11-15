import java.util.Arrays;

public class Sorting {
    static int[] numbers() {
        return new int[] {5, 3, 9, 1, 7, 2, 8, 6, 4, 0};
    }

    static int[] insertionSort(int[] a) {
        for (int i = 1; i < a.length; i++) {
            int x = a[i];
            int j = i - 1;
            while (j >= 0 && a[j] > x) {
                a[j + 1] = a[j];
                j--;
            }
            a[j + 1] = x;
        }
        return a;
    }

    static long[][] grid(int rows, int columns) {
        long[][] grid = new long[rows][columns];
        for (int i = 0; i < rows; i++) {
            for (int j = 0; j < columns; j++) {
                grid[i][j] = (long) i * columns + j;
            }
        }
        return grid;
    }

    static Object cloned(int[] a) {
        return a.clone();
    }

    static int[] shifted(int[] a) {
        System.arraycopy(a, 0, a, 1, a.length - 1);
        return a;
    }

    static char[] letters(String s) {
        return s.toCharArray();
    }

    static void copyTooFar(int[] a) {
        System.arraycopy(a, 0, a, 5, 10);
    }

    static void copyMismatch() {
        System.arraycopy(new int[1], 0, new long[1], 0, 1);
    }

    static int pastTheEnd(int[] a) {
        return a[a.length];
    }

    static int[] negative(int n) {
        return new int[n];
    }

    // Runs only on HotSpot.
    public static void main(String[] args) {
        System.out.println(Arrays.toString(insertionSort(numbers())));
        System.out.println(Arrays.deepToString(grid(2, 3)));
        System.out.println(Arrays.toString((int[]) cloned(numbers())));
        System.out.println(Arrays.toString(shifted(numbers())));
        System.out.println(Arrays.toString(letters("héllo")));
        Runnable[] failures = {
            () -> copyTooFar(numbers()),
            () -> copyMismatch(),
            () -> pastTheEnd(numbers()),
            () -> negative(-1),
        };
        for (Runnable failure : failures) {
            try {
                failure.run();
            } catch (RuntimeException e) {
                System.out.println(e);
            }
        }
    }
}
