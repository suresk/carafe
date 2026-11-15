public class Catches {
    static int parse(String s) {
        try {
            return Integer.parseInt(s);
        } catch (NumberFormatException e) {
            return -1;
        } finally {
            System.out.println("done");
        }
    }

    static void either(Object o) {
        try {
            o.hashCode();
        } catch (NullPointerException | ClassCastException e) {
            System.out.println("caught");
        }
    }
}
