public class Init {
    // Runs only on HotSpot: prints what each scenario leaves in the trace.
    public static void main(String[] args) {
        System.out.println(superclassFirst());
        System.out.println(constantsAreNotAccesses());
        System.out.println(cycle());
        System.out.println(interfaces());
        for (int i = 0; i < 2; i++) {
            try {
                System.out.println(failing());
            } catch (Throwable t) {
                System.out.println(t);
            }
        }
    }

    static long superclassFirst() {
        Trace.reset();
        int z = Derived.z;
        Trace.mark(z);
        return Trace.trace;
    }

    static long constantsAreNotAccesses() {
        Trace.reset();
        Trace.mark(Constants9.K);
        Trace.mark(Constants9.L);
        return Trace.trace;
    }

    static long cycle() {
        Trace.reset();
        Trace.mark(Ping.p);
        Trace.mark(Pong.q);
        return Trace.trace;
    }

    static long interfaces() {
        Trace.reset();
        new Implementer();
        return Trace.trace;
    }

    static long inheritedField() {
        Trace.reset();
        Trace.mark(Derived.x);
        return Trace.trace;
    }

    static int failing() {
        return Fragile.value;
    }
}

class Trace {
    static long trace;

    static void reset() {
        trace = 0;
    }

    static int mark(int n) {
        trace = trace * 10 + n;
        return n;
    }
}

class Base {
    static int x = Trace.mark(1);
}

class Derived extends Base {
    static int y = Trace.mark(2);
    static int z = 3;
}

class Constants9 {
    static final int K = 4;
    static final int L = Trace.mark(5);
}

class Ping {
    static int p = Pong.q + 1;
}

class Pong {
    static int q = Ping.p + 1;
}

interface WithDefault {
    int W = Trace.mark(6);

    default int w() {
        return W;
    }
}

interface WithoutDefault {
    int V = Trace.mark(7);
}

class Implementer implements WithDefault, WithoutDefault {
    static int i = Trace.mark(8);
}

class Fragile {
    static int value = 1 / Trace.mark(0);
}
