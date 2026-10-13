package shop.books;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import static java.util.Objects.requireNonNull;

/**
 * A ledger keeps entries in the order they were posted.
 *
 * @param <T> the kind of amount
 */
@SuppressWarnings({"unchecked", "rawtypes"})
public final class Ledger<T extends Number & Comparable<T>> extends Base implements Book, AutoCloseable {
    public static final int LIMIT = 1_000;
    private static final String HEADER = "memo,cents\n";
    private final List<Entry> entries = new ArrayList<>();
    private volatile transient int reads;
    protected char sep = ',';

    public enum Kind { DEBIT, CREDIT }

    public record Entry(String memo, long cents, Kind kind) {
        public Entry {
            requireNonNull(memo);
        }
    }

    sealed interface Event permits Posted, Voided {}

    @FunctionalInterface
    interface Rule {
        boolean test(Entry e) throws IllegalStateException;
    }

    public Ledger(String name) {
        super(name);
        this.reads = 0;
    }

    @Override
    public long total() {
        long sum = 0L;
        for (Entry e : entries) {
            sum += e.kind() == Kind.CREDIT ? -e.cents() : e.cents();
        }
        return sum;
    }

    public Optional<Entry> find(final String memo) {
        return entries.stream()
                .filter(e -> e.memo().equals(memo))
                .map(Entry.class::cast)
                .findFirst();
    }

    public synchronized void post(Entry e) {
        try {
            if (e.cents() < 0) throw new IllegalArgumentException("negative");
            entries.add(e);
        } catch (IllegalArgumentException | NullPointerException err) {
            System.err.printf("post: %s%n", err.getMessage());
        } finally {
            reads++;
        }
    }

    String describe(Object o) {
        return switch (o) {
            case Integer i when i > 0 -> "positive " + i;
            case String s -> String.format("text %-10s %d", s, s.length());
            case null, default -> {
                yield "other";
            }
        };
    }

    static <A> A first(List<? super A> list, A fallback) {
        var text = """
            memo,cents
            cash,100
            """;
        double d = 0x1Fp3 + 1e-3 + 0b1010 + 017 + 3.5f;
        int[] seen = new int[] {1, 2, 3};
        assert seen.length == 3 : "three";
        outer:
        for (int i = 0; i < seen.length; i++) {
            if (seen[i] instanceof Integer n && n > 1) {
                continue outer;
            }
            while (true) break;
        }
        return list.isEmpty() || text == null ? fallback : null;
    }

    @Override
    public void close() {}
}
