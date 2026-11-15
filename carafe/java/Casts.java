import java.util.ArrayList;
import java.util.List;

public class Casts {
    static void interfaces() {
        Named[] things = {new Robot(), new Person()};
        for (Named thing : things) {
            System.out.println(thing.name());
            System.out.println(thing.greeting());
        }
        System.out.println(new Ambidextrous().side());
    }

    static void casts() {
        Object text = "text";
        System.out.println(text instanceof Comparable ? "comparable" : "not comparable");
        System.out.println(text instanceof Named ? "named" : "not named");
        Object robot = new Robot();
        Named named = (Named) robot;
        System.out.println(named.name());
        Object numbers = new int[3];
        System.out.println(numbers instanceof Object[] ? "Object[]" : "not Object[]");
        System.out.println(numbers instanceof Cloneable ? "Cloneable" : "not Cloneable");
        Object robots = new Robot[2];
        System.out.println(robots instanceof Named[] ? "Named[]" : "not Named[]");
    }

    static void list() {
        List<String> names = new ArrayList<>(10);
        names.add("carafe");
        names.add("ember");
        names.add("felite");
        for (String name : names) {
            System.out.println(name);
        }
        System.out.println(names.contains("ember") ? "contains ember" : "no ember");
    }

    static void badCast() {
        Object robot = new Robot();
        String s = (String) robot;
    }

    static void badArrayCast() {
        Object array = new Robot[1];
        String[] strings = (String[]) array;
    }

    static void badStore() {
        Object[] strings = new String[1];
        strings[0] = new Robot();
    }

    static void badCopy() {
        Object[] mixed = {"a", new Robot()};
        System.arraycopy(mixed, 0, new String[2], 0, 2);
    }

    static void viaLeft() {
        Left left = new Ambidextrous();
        System.out.println(left.side());
    }

    // Runs only on HotSpot: the printing methods, then each failure.
    public static void main(String[] args) {
        interfaces();
        casts();
        list();
        Runnable[] failures = {Casts::badCast, Casts::badArrayCast, Casts::badStore,
            Casts::badCopy};
        for (Runnable failure : failures) {
            try {
                failure.run();
            } catch (RuntimeException e) {
                System.out.println(e);
            }
        }
    }
}

interface Named {
    String name();

    default String greeting() {
        return "Hello";
    }
}

class Robot implements Named {
    public String name() {
        return "robot";
    }
}

class Person implements Named {
    public String name() {
        return "person";
    }

    public String greeting() {
        return "Hi there";
    }
}

interface Left {
    default String side() {
        return "left";
    }
}

interface Right {
    default String side() {
        return "right";
    }
}

class Ambidextrous implements Left, Right {
    public String side() {
        return Left.super.side();
    }
}
