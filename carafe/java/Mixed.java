public class Mixed {
    byte b;
    long l;
    Object o;
    short s;
    double d;
    int i;
    boolean z;
    char c;
    static int count;

    static Mixed make() {
        Mixed m = new Mixed();
        m.b = -1;
        m.l = 0x0102030405060708L;
        m.s = 300;
        m.d = 1.0;
        m.i = 7;
        m.z = true;
        m.c = 'A';
        m.o = m;
        return m;
    }

    static int narrow(Mixed m, int x) {
        m.b = (byte) x;
        return m.b;
    }
}
