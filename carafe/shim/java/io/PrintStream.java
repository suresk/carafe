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
    // @hide

    public void print(Object o) {
        print(String.valueOf(o));
    }

    public void print(boolean b) {
        print(String.valueOf(b));
    }

    public void print(char c) {
        print(String.valueOf(c));
    }

    public void print(int i) {
        print(String.valueOf(i));
    }

    public void print(long l) {
        print(String.valueOf(l));
    }

    public void print(float f) {
        print(String.valueOf(f));
    }

    public void print(double d) {
        print(String.valueOf(d));
    }

    public void print(char[] s) {
        print(new String(s));
    }

    public void println(Object o) {
        print(o);
        println();
    }

    public void println(boolean b) {
        print(b);
        println();
    }

    public void println(char c) {
        print(c);
        println();
    }

    public void println(int i) {
        print(i);
        println();
    }

    public void println(long l) {
        print(l);
        println();
    }

    public void println(float f) {
        print(f);
        println();
    }

    public void println(double d) {
        print(d);
        println();
    }

    public void println(char[] s) {
        print(s);
        println();
    }
    // @show
}
// @end
