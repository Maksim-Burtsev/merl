package typeargs;

public interface Specs {
    static <E extends Build, K extends Comparable<K>> Builder<E, K> single(
            String path, Class<K> type) {
        return null;
    }

    interface Builder<E, K> {
    }
}
