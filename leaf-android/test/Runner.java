package leaf;

import leaf.Leaf;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.Paths;
public class Runner implements Leaf.Callback {
    public static void main(String[] args) {
        new Runner().run();
    }

    void run() {
        String conf = """
[General]
loglevel = debug
dns-server = 114.114.114.114,223.5.5.5
interface = 127.0.0.1
port = 20801
[Proxy]
Direct = direct
""";
        try {
            File cfgFile = File.createTempFile("config-", ".conf");
            Files.write(Paths.get(cfgFile.getPath()), conf.getBytes());
            (new Thread(() -> { Leaf.run(0, cfgFile.getPath(), this); })).start();
            try { System.in.read(); } catch(Exception e) {}
            System.out.println("leaf exit...");
            Leaf.shutdown(0);
            try { Thread.sleep(1000); } catch(Exception e) { e.printStackTrace(); }
            System.out.println("leaf exited");
        } catch (Exception e) {
            e.printStackTrace();
        }
    }

    public void reportState(int state) {
        System.out.println("leaf state: %d".formatted(state));
    }

    public void reportTraffic(int txRate, int rxRate, long txTotal, long rxTotal) {
        System.out.println("txRate: %d, rxRate: %d, txTotal: %d, rxTotal: %d".formatted(txRate, rxRate, txTotal, rxTotal));
    }
}