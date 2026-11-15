public class Suppressed {
    static final class Resource implements AutoCloseable {
        private final String name;

        Resource(String name) {
            this.name = name;
        }

        @Override
        public void close() {
            throw new IllegalStateException(name);
        }
    }

    static void use() {
        try (Resource a = new Resource("a"); Resource b = new Resource("b")) {
            throw new UnsupportedOperationException("body");
        }
    }

    public static void main(String[] args) {
        try {
            use();
        } catch (UnsupportedOperationException e) {
            throw new RuntimeException("use failed", e);
        }
    }
}
