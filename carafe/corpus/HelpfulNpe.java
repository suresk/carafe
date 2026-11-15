public class HelpfulNpe {
    static String name;
    public static void main(String[] args) {
        try { name.length(); } catch (NullPointerException e) { System.out.println(e.getMessage()); }
    }
}
