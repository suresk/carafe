import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.Map;

public class Reflect {
    enum Planet {
        MERCURY, VENUS, EARTH, MARS;

        boolean inner() {
            return ordinal() < 4;
        }
    }

    public static class Counter {
        private int count;

        public Counter() {
        }

        public Counter(int start) {
            count = start;
        }

        public int add(int n) {
            count += n;
            return count;
        }

        public static String shout(String s) {
            return s.toUpperCase() + "!";
        }

        public void fail() {
            throw new IllegalStateException("failed on purpose");
        }
    }

    public static void main(String[] args) throws Exception {
        System.out.println(Planet.valueOf("EARTH").ordinal());
        try {
            Planet.valueOf("PLUTO");
        } catch (IllegalArgumentException e) {
            System.out.println(e.getMessage());
        }
        EnumMap<Planet, Integer> moons = new EnumMap<>(Planet.class);
        moons.put(Planet.MARS, 2);
        moons.put(Planet.EARTH, 1);
        System.out.println(moons);
        EnumSet<Planet> rocky = EnumSet.range(Planet.VENUS, Planet.MARS);
        System.out.println(rocky + " " + EnumSet.complementOf(rocky));

        Class<?> counter = Class.forName("Reflect$Counter");
        Object c = counter.getDeclaredConstructor(int.class).newInstance(40);
        Method add = counter.getMethod("add", int.class);
        System.out.println(add);
        System.out.println(add.invoke(c, 2));
        Method shout = counter.getMethod("shout", String.class);
        System.out.println(shout.invoke(null, "hello"));
        System.out.println(Modifier.toString(shout.getModifiers()));
        String[] names = Arrays.stream(counter.getDeclaredMethods())
                .map(Method::getName).sorted().toArray(String[]::new);
        System.out.println(String.join(" ", names));
        Constructor<?>[] constructors = counter.getConstructors();
        System.out.println(constructors.length);
        try {
            counter.getMethod("fail").invoke(c);
        } catch (InvocationTargetException e) {
            System.out.println(e.getCause());
        }
        try {
            add.invoke(c, "two");
        } catch (IllegalArgumentException e) {
            System.out.println(e.getMessage());
        }
        try {
            Class.forName("Missing");
        } catch (ClassNotFoundException e) {
            System.out.println(e);
        }
    }
}
