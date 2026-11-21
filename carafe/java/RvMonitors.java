/// Monitors with one thread: ownership is still checked.
public class RvMonitors {
    static synchronized void s() {
        RvMonitors.class.notify();
        System.out.println("static sync notify ok");
    }

    synchronized void i() throws Exception {
        notifyAll();
        wait(1);
        System.out.println("sync wait(1) ok");
    }

    synchronized void overwrite() {
        RvMonitors self = this;
        self.notify();
        System.out.println("held after astore ok");
    }

    static synchronized void fail() {
        throw new IllegalStateException("thrown from a synchronized method");
    }

    public static void main(String[] a) throws Exception {
        Object o = new Object();
        try {
            o.notify();
        } catch (IllegalMonitorStateException e) {
            System.out.println("notify: " + e);
        }
        try {
            o.notifyAll();
        } catch (IllegalMonitorStateException e) {
            System.out.println("notifyAll: " + e);
        }
        try {
            o.wait(1);
        } catch (IllegalMonitorStateException e) {
            System.out.println("wait: " + e);
        }
        synchronized (o) {
            o.notify();
            System.out.println("held notify ok");
        }
        synchronized (o) {
            synchronized (o) {
                o.notify();
            }
            o.notify();
            System.out.println("nested ok");
        }
        s();
        new RvMonitors().i();
        new RvMonitors().overwrite();
        try {
            fail();
        } catch (IllegalStateException e) {
            System.out.println(e.getMessage());
        }
        try {
            RvMonitors.class.notify();
        } catch (IllegalMonitorStateException e) {
            System.out.println("released: " + e);
        }
        try {
            o.wait(-1);
        } catch (Exception e) {
            System.out.println("wait(-1): " + e);
        }
    }
}
