public class Dispatch {
    public static void main(String[] args) {
        Animal animal = new Animal();
        Animal dog = new Dog();
        Animal puppy = new Puppy();
        animal.speak();
        dog.speak();
        puppy.speak();
        puppy.origin();
        dog.secretly();
        puppy.describe();
        Inner inner = new Inner();
        inner.callOuter(new Dispatch());
        Shape2 shape = new Circle2();
        System.out.println(shape.kind());
    }

    private void whisper() {
        System.out.println("Dispatch.whisper");
    }

    static class Inner {
        void callOuter(Dispatch outer) {
            outer.whisper();
        }
    }
}

class Animal {
    Animal() {
        System.out.println("Animal()");
    }

    void speak() {
        System.out.println("...");
    }

    void origin() {
        System.out.println("Animal.origin");
    }

    void secretly() {
        secret();
    }

    private void secret() {
        System.out.println("Animal.secret");
    }

    void describe() {
        speak();
    }
}

class Dog extends Animal {
    Dog() {
        System.out.println("Dog()");
    }

    void speak() {
        System.out.println("Woof");
    }

    void origin() {
        System.out.println("Dog.origin");
        super.origin();
    }

    private void secret() {
        System.out.println("Dog.secret");
    }
}

class Puppy extends Dog {
    Puppy() {
        System.out.println("Puppy()");
    }

    void speak() {
        System.out.println("Yip");
    }

    void origin() {
        System.out.println("Puppy.origin");
        super.origin();
    }
}

abstract class Shape2 {
    abstract String kind();
}

class Circle2 extends Shape2 {
    String kind() {
        return "circle";
    }
}
