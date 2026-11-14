package quern.bins;

import quern.Stone;

public class Shelf {
    private final TreeMap<String, Stone> byName = new TreeMap<>();

    public int sum() {
        return byName.get("a").turn();
        //                     ^ d: !jump
    }
}
