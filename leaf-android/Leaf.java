package leaf;

public class Leaf {
    interface Callback {
        void reportTraffic(float txRate, float rxRate, long txTotal, long rxTotal);
    }

    public static native int runWithOptions(
            int rtId,
            final String configPath,
            Callback callback,
            boolean autoReload,
            boolean multiThread,
            boolean autoThreads,
            int threads,
            int stackSize);

    public static native int run(int rtId, final String configPath, Callback callback);

    public static native boolean reload(int rtId);

    public static native boolean shutdown(int rtId);

    public static native boolean testConfigdown(final String configPath);

    static {
        System.loadLibrary("leaf");
    }
}