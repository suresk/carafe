public class Exceptions {
    static final class Resource implements AutoCloseable {
        private final String name;
        private final boolean fails;

        Resource(String name, boolean fails) {
            this.name = name;
            this.fails = fails;
        }

        @Override
        public void close() {
            System.out.print("close ");
            System.out.println(name);
            if (fails) {
                throw new IllegalStateException(name);
            }
        }
    }

    static int divide(int a, int b) {
        return a / b;
    }

    static int depth = 0;

    static void recurse() {
        depth++;
        recurse();
    }

    static String finallyOrder() {
        StringBuilder order = new StringBuilder();
        for (int i = 0; i < 3; i++) {
            try {
                if (i == 1) {
                    continue;
                }
                order.append('t');
            } finally {
                order.append(i);
            }
        }
        return order.toString();
    }

    public static void main(String[] args) {
        try {
            divide(1, 0);
        } catch (ArithmeticException e) {
            System.out.println(e.getMessage());
        }
        try {
            int[] a = new int[2];
            a[5] = 1;
        } catch (ArrayIndexOutOfBoundsException e) {
            System.out.println(e.getMessage());
        }
        try {
            Object o = "s";
            Integer i = (Integer) o;
        } catch (ClassCastException e) {
            System.out.println(e.getMessage());
        }
        try {
            String n = null;
            n.length();
        } catch (NullPointerException e) {
            System.out.println(e.getMessage());
        }
        try {
            throw new IllegalStateException("bad");
        } catch (RuntimeException e) {
            System.out.println(e);
        } finally {
            System.out.println("finally");
        }
        try (Resource a = new Resource("a", false); Resource b = new Resource("b", true)) {
            System.out.println("body");
            throw new UnsupportedOperationException("body");
        } catch (UnsupportedOperationException e) {
            System.out.println(e.getSuppressed()[0]);
        }
        try {
            Integer.parseInt("x1");
        } catch (NumberFormatException e) {
            System.out.println(e.getMessage());
        }
        try {
            recurse();
        } catch (StackOverflowError e) {
            System.out.println(depth > 1000);
        }
        System.out.println(finallyOrder());
        Throwable t = new Throwable();
        System.out.println(t.getCause());
    }
}
