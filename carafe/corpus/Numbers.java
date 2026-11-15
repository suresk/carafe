public class Numbers {
    public static void main(String[] args) {
        System.out.println(Integer.parseInt("-123") + Long.parseLong("9000000000"));
        System.out.println(Double.parseDouble("3.25") * 2 + " " + Float.parseFloat("1.5"));
        System.out.println(Integer.toBinaryString(10) + " " + Integer.toHexString(255) + " " + Long.MAX_VALUE);
        System.out.println(Math.sqrt(2) + " " + Math.abs(-7) + " " + Math.max(3, 9) + " " + Math.floorMod(-7, 3));
        System.out.println(Integer.MAX_VALUE + 1 + " " + (char) ('a' + 2) + " " + 7 / 2.0 + " " + (long) 1e18);
        System.out.println(Integer.valueOf(127) == Integer.valueOf(127));
        System.out.println(Character.isDigit('7') + " " + Character.toUpperCase('q') + " " + Character.getNumericValue('9'));
    }
}
