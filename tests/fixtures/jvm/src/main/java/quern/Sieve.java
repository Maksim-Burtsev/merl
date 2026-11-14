package quern;

import java.util.*;

public class Sieve {
    private final Vector<Stone> stones = new Vector<>();

    public int sum() {
        return stones.get(0).turn();
        //                   ^ d: src/main/java/quern/Stone.java:4
    }
}
