import java.math.BigInteger;

public class BigNumbers {
    public static void main(String[] args) {
        BigInteger f = BigInteger.ONE;
        for (int i = 2; i <= 25; i++) f = f.multiply(BigInteger.valueOf(i));
        System.out.println(f);
    }
}
