package quern;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public class Bins {
    private final Map<String, Stone> byName = new HashMap<>();
    private final Map<String, List<Stone>> rows = new HashMap<>();
    private final Optional<Stone> spare = Optional.empty();

    public int sum() {
        int total = byName.get("a").turn();
        //                          ^ d: src/main/java/quern/Stone.java:4
        total += byName.getOrDefault("a", null).turn();
        //                                      ^ d: src/main/java/quern/Stone.java:4
        total += rows.get("a").get(0).turn();
        //                            ^ d: !jump
        return total + spare.orElseThrow().turn();
        //                                 ^ d: src/main/java/quern/Stone.java:4
    }
}
