public class Boxing {
    public static void main(String[] args) {
        Integer small = 100;
        Integer big = 1000;
        System.out.println(small);
        System.out.println(big);
        System.out.println(small == Integer.valueOf(100) ? "cached" : "not cached");
        System.out.println(big == Integer.valueOf(1000) ? "cached" : "not cached");
        Long l = 5L;
        System.out.println(l.hashCode());
        System.out.println(Character.toUpperCase('q'));
        System.out.println(Integer.toHexString(255));
        System.out.println(Long.toString(-123456789012L));
    }
}
