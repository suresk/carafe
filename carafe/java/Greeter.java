public class Greeter {
    String greeting() {
        return "Hello";
    }

    String name() {
        return "greeter";
    }

    static native long ticks();

    static long now() {
        return ticks();
    }

    public static void main(String[] args) {
        Greeter quiet = new Greeter();
        Greeter loud = new Loud();
        System.out.println(quiet.greeting());
        System.out.println(loud.greeting());
        System.out.println(loud.name());
        System.out.println("Grüße, 世界! 😀");
        System.out.println(quiet.greeting() == loud.name() ? "same" : "different");
    }
}

class Loud extends Greeter {
    String greeting() {
        return "HELLO";
    }
}
