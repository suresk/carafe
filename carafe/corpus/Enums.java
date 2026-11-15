public class Enums {
    enum Color { RED, GREEN, BLUE; Color next() { return values()[(ordinal() + 1) % values().length]; } }

    static String describe(Color c) {
        switch (c) {               // classic enum switch: a synthetic $SwitchMap, no indy
            case RED: return "warm";
            case BLUE: return "cool";
            default: return "neutral";
        }
    }

    public static void main(String[] args) {
        for (Color c : Color.values()) System.out.println(c + " " + c.ordinal() + " " + describe(c));
        System.out.println(Color.RED.next().name());
        System.out.println(Color.valueOf("BLUE").compareTo(Color.RED));
    }
}
