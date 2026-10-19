package typeuse;

public final class Registry {
    public static <T> @Nullable T lookup(String path) {
        return null;
    }

    public static @Size(max = 3) String code(String path) {
        return path;
    }
}
