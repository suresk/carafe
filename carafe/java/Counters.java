import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

public class Counters {
    public static void main(String[] args) {
        AtomicInteger count = new AtomicInteger();
        for (int i = 0; i < 5; i++) {
            count.incrementAndGet();
        }
        System.out.println(count.get());
        System.out.println(count.compareAndSet(5, 50));
        System.out.println(count.compareAndSet(5, 60));
        System.out.println(count);
        AtomicLong total = new AtomicLong(1L << 40);
        System.out.println(total.addAndGet(-1));
        System.out.println(Float.floatToIntBits(1.5f));
        System.out.println(Double.doubleToLongBits(-0.0));
        System.out.println(Integer.valueOf(127) == Integer.valueOf(127));
    }
}
