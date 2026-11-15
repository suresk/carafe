package java.lang;

import java.io.PrintStream;

// @listing system-shim
/**
 * carafe's System, which replaces the JDK's: standard output and standard
 * error, as streams on file descriptors 1 and 2, with none of the JDK's
 * startup behind them.
 */
public final class System {
    private System() {}

    public static final PrintStream out = new PrintStream(1);
    public static final PrintStream err = new PrintStream(2);
}
// @end
