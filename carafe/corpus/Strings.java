public class Strings {
    public static void main(String[] args) {
        String s = "  Hello, World  ";
        String t = s.trim();
        System.out.println(t.toUpperCase() + " " + t.toLowerCase());
        System.out.println(t.substring(7) + " " + t.indexOf("World") + " " + t.charAt(4));
        System.out.println(t.replace('l', 'L') + " " + t.contains("lo,") + " " + t.startsWith("He"));
        System.out.println(String.join("-", t.split(", ")) + " " + "ab".repeat(3));
        System.out.println("b".compareTo("a") + " " + "Hi".equalsIgnoreCase("hI"));
        System.out.println(new StringBuilder("stressed").reverse() + " " + new StringBuilder("ac").insert(1, 'b'));
        System.out.println("café λ".length() + " " + "λ".toUpperCase());
        char[] cs = t.toCharArray();
        System.out.println(cs.length + " " + String.valueOf(cs, 0, 5) + " " + Character.isLetter('x'));
    }
}
