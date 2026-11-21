/// `getModifiers` of arrays of nested classes, and of a lambda's class.
public class RvMods {
    protected static class Prot {}
    private static class Priv {}

    public static void main(String[] a) {
        System.out.println(Integer.toHexString(Prot[].class.getModifiers()));
        System.out.println(Integer.toHexString(Priv[].class.getModifiers()));
        System.out.println(Integer.toHexString(Prot[][].class.getModifiers()));
        System.out.println(Integer.toHexString(Prot.class.getModifiers()));
        System.out.println(Integer.toHexString(int[].class.getModifiers()));
        Runnable r = () -> {};
        System.out.println(Integer.toHexString(r.getClass().getModifiers()));
    }
}
