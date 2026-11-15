public class Overflow {
    static int down(int n) {
        return down(n + 1) + 1;
    }

    public static void main(String[] args) {
        down(0);
    }
}
