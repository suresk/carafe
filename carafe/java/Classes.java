import java.util.ArrayList;
import java.util.List;
import java.util.RandomAccess;

public class Classes {
    public static void main(String[] args) {
        Object number = 42;
        System.out.println(Integer.class.isInstance(number));
        System.out.println(String.class.isInstance(number));
        System.out.println(Number.class.isAssignableFrom(Integer.class));
        System.out.println(Integer.class.isAssignableFrom(Number.class));
        System.out.println(int[].class.isArray());
        System.out.println(List.class.isInterface());
        System.out.println(int.class.isPrimitive());
        System.out.println(Integer.class.getSuperclass().getName());
        System.out.println(List.class.getSuperclass());
        System.out.println(String[][].class.getComponentType().getName());
        System.out.println(new ArrayList<String>() instanceof RandomAccess);
        System.out.println(ArrayList.class);
        System.out.println(Runnable.class);
        System.out.println(Classes.class == new Classes().getClass());
    }
}
