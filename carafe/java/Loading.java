public class Loading {
    public static void main(String[] args) {
        int area = windowArea(3);
        if (area < 0) {
            new Unused();
        }
    }

    static int windowArea(int side) {
        return Geometry.area(new Window(side));
    }
}

class Geometry {
    static int area(Rectangle rectangle) {
        return rectangle.width * rectangle.height;
    }
}

abstract class Figure {
    int corners;

    Figure(int corners) {
        this.corners = corners;
    }
}

interface Labeled {
}

class Rectangle extends Figure implements Labeled {
    int width;
    int height;

    Rectangle(int width, int height) {
        super(4);
        this.width = width;
        this.height = height;
    }
}

class Window extends Rectangle {
    Window(int side) {
        super(side, side);
    }
}

class Unused {
}
