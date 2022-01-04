package leaf;

public class Leaf {
    public static native int runWithOptions(
            int rtId,
            final String configPath,
//            boolean autoReload,
            boolean multiThread,
            boolean autoThreads,
            int threads,
            int stackSize);

    public static native int run(int rtId, final String configPath);

    public static native boolean reload(int rtId);

    public static native boolean shutdown(int rtId);

    public static native boolean testConfigdown(final String configPath);

    static {
        System.loadLibrary("leaf");
    }
}