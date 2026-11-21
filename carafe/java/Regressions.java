// Linkage cases that tests/regressions.rs checks against HotSpot. The test
// patches some of these class files first, as the comments say, so that they
// break rules javac would not let the source break.
public class Regressions {
    public static void main(String[] args) {
        Runnable[] cases = {
            () -> System.out.println(new Bottom().seven()),
            () -> new FromFinal(),
            () -> new Implementor(),
            () -> System.out.println(Caller.call()),
            () -> new p.Sub(),
            () -> new p.Impl(),
            () -> new Holder().set(1),
            () -> Outsider.set(new Holder()),
            () -> Holder.bump(),
            () -> Outsider.bump(),
            () -> System.out.println(p.Same.cast(p.Same.make())),
            () -> System.out.println(q.Peek.cast(null)),
            () -> System.out.println(q.Peek.cast(new Object[0])),
            () -> System.out.println(MissingUser.cast()),
            () -> System.out.println(MissingUser.test()),
            () -> new FaceImpl().callRun(),
            () -> new FaceImpl().callDefault(),
            () -> new SubS().callSuper(),
        };
        for (Runnable c : cases) {
            try {
                c.run();
                System.out.println("ok");
            } catch (Throwable t) {
                System.out.println(t);
            }
        }
    }
}

// A default method inherited through a superclass (JVMS 5.4.3.3).
interface Defaulted {
    default int seven() {
        return 7;
    }
}

class Middle implements Defaulted {}

class Bottom extends Middle {}

// Patched to be final.
class FinalBase {}

class FromFinal extends FinalBase {}

// Patched to be a class.
interface NotAnInterface {}

class Implementor implements NotAnInterface {}

class Secretive {
    // Patched to be private.
    static int m() {
        return 1;
    }
}

class Caller {
    static int call() {
        return Secretive.m();
    }
}

class Holder {
    // Both patched to be final.
    int value;
    static int count;

    void set(int v) {
        value = v;
    }

    static void bump() {
        count++;
    }
}

class Outsider {
    static void set(Holder h) {
        h.value = 2;
    }

    static void bump() {
        Holder.count = 3;
    }
}

// Deleted from the class path.
class Missing {}

class MissingUser {
    static Object nothing() {
        return null;
    }

    static String cast() {
        Missing m = (Missing) nothing();
        return "cast " + (m == null);
    }

    static boolean test() {
        return nothing() instanceof Missing;
    }
}

interface Face {
    void run();

    // Patched to be static.
    default void other() {}
}

class FaceImpl implements Face {
    // Patched to be package-private.
    public void run() {}

    void callRun() {
        Face f = this;
        f.run();
    }

    void callDefault() {
        Face f = this;
        f.other();
    }
}

class SuperS {
    // Patched to be static.
    void m() {}
}

class SubS extends SuperS {
    void callSuper() {
        super.m();
    }
}
