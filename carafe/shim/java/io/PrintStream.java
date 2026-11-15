package java.io;

// @listing print-stream-shim
/**
 * carafe's PrintStream: text written straight to a file descriptor, through
 * one native method. The line separator is always "\n".
 */
public class PrintStream {
    private final int fd;

    public PrintStream(int fd) {
        this.fd = fd;
    }

    private static native void write0(int fd, String s);

    public void print(String s) {
        write0(fd, s == null ? "null" : s);
    }

    public void println() {
        print("\n");
    }

    public void println(String s) {
        print(s);
        println();
    }

    public void flush() {}
}
// @end
