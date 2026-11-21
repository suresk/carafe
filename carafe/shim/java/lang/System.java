// @listing system-shim
package java.lang;

import java.io.PrintStream;

/**
 * carafe's System, which replaces the JDK's: standard output and standard
 * error, as streams on file descriptors 1 and 2, with none of the JDK's
 * startup behind them.
 */
public final class System {
    private System() {}

    // @hide
    /**
     * What HotSpot's startup does before main that the class library needs:
     * saving the system properties that jdk.internal.misc.VM hands out.
     * carafe's run_main calls it first.
     */
    static void startup() {
        jdk.internal.misc.VM.saveProperties(new java.util.HashMap<>());
        jdk.internal.access.SharedSecrets.setJavaLangAccess(new LangAccess());
        in = new In();
    }

    /** The system properties: fixed values, the same on every platform. */
    private static final class Props {
        static final java.util.Properties PROPERTIES = new java.util.Properties();

        static {
            String[][] fixed = {
                {"java.home", "/carafe"}, {"user.home", "/home/carafe"},
                {"user.dir", "/home/carafe"}, {"user.name", "carafe"},
                {"java.io.tmpdir", "/tmp"}, {"native.encoding", "UTF-8"},
                {"file.encoding", "UTF-8"}, {"sun.jnu.encoding", "UTF-8"},
                {"stdout.encoding", "UTF-8"}, {"stderr.encoding", "UTF-8"},
                {"os.name", "carafe"}, {"os.arch", "carafe"}, {"os.version", "1"},
                {"user.language", "en"}, {"line.separator", "\n"},
                {"file.separator", "/"}, {"path.separator", ":"},
            };
            for (String[] property : fixed) {
                PROPERTIES.setProperty(property[0], property[1]);
            }
        }
    }

    public static java.util.Properties getProperties() {
        return Props.PROPERTIES;
    }

    public static String getProperty(String key) {
        return Props.PROPERTIES.getProperty(key);
    }

    public static String getProperty(String key, String fallback) {
        return Props.PROPERTIES.getProperty(key, fallback);
    }

    public static String lineSeparator() {
        return "\n";
    }

    // @show
    public static final PrintStream out = new PrintStream(1);
    public static final PrintStream err = new PrintStream(2);
    // @hide

    public static native void arraycopy(Object src, int srcPos, Object dest, int destPos,
                                        int length);

    public static native int identityHashCode(Object x);

    public static native long nanoTime();

    public static native long currentTimeMillis();

    /** Standard input, which startup sets. */
    public static java.io.InputStream in;

    /**
     * Standard input as carafe holds it: bytes in the VM, read by one native,
     * with no file descriptors.
     */
    private static final class In extends java.io.InputStream {
        @Override
        public int read() {
            byte[] one = new byte[1];
            return read(one, 0, 1) == 1 ? one[0] & 0xff : -1;
        }

        @Override
        public int read(byte[] b, int off, int len) {
            java.util.Objects.checkFromIndexSize(off, len, b.length);
            return len == 0 ? 0 : read0(b, off, len);
        }

        /** Copies up to len bytes into b at off; returns how many, or -1 at the end. */
        private static native int read0(byte[] b, int off, int len);
    }
    // @show
}
// @end
