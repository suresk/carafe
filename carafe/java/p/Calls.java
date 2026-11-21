package p;

public class Calls {
    public static void run() {
        System.out.println(((A) new q.C()).m()); // A.m on a C
        Z z = new Z();
        System.out.println(((X) z).m()); // X.m on a Z
        q.Calls.run(z); // Y.m on a Z
        X y = new q.Y();
        System.out.println(y.m()); // X.m on a Y
    }
}
