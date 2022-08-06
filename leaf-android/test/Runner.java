package leaf;

import leaf.Leaf;

public class Runner implements Leaf.Callback {
    public static void main(String[] args) {
        new Runner().run();
    }

    void run() {
        (new Thread(() -> { Leaf.run(0, "configFile.absolutePath", this); })).start();
        try { System.in.read(); } catch(Exception e) {}
        System.out.println("leaf exit...");
        Leaf.shutdown(0);
        try { Thread.sleep(1000); } catch(Exception e) { e.printStackTrace(); }
        System.out.println("leaf exited");
    }

    public void reportState(int state) {
        System.out.println("leaf state: %d".formatted(state));
    }

    public void reportTraffic(float txRate, float rxRate, long txTotal, long rxTotal) {
        System.out.println("txRate: %d, rxRate: $d, txTotal: %d, rxTotal: %d".formatted(txRate, rxRate, txTotal, rxTotal));
    }
}