public class StringSwitch {
    static int score(String word) {
        switch (word) {
            case "apple": return 1;
            case "banana": return 2;
            default: return 0;
        }
    }

    static String arrow(String s) {
        return switch (s) { case "a", "b" -> "early"; case "z" -> "late"; default -> "middle"; };
    }

    public static void main(String[] args) {
        System.out.println(score("apple") + score("banana") + score("cherry"));
        System.out.println(arrow("a") + " " + arrow("m") + " " + arrow("z"));
    }
}
