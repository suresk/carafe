public class Node {
    int value;
    Node next;

    Node(int value, Node next) {
        this.value = value;
        this.next = next;
    }

    static Node range(int n) {
        Node list = null;
        for (int i = n; i > 0; i--) {
            list = new Node(i, list);
        }
        return list;
    }

    static int sum(Node list) {
        int total = 0;
        for (Node node = list; node != null; node = node.next) {
            total += node.value;
        }
        return total;
    }

    static Node reverse(Node list) {
        Node done = null;
        while (list != null) {
            Node rest = list.next;
            list.next = done;
            done = list;
            list = rest;
        }
        return done;
    }

    static boolean same(Node a, Node b) {
        return a == b;
    }

    static int first(Node list) {
        return list.value;
    }
}
