package tagine;

import java.util.function.Consumer;
import java.util.stream.Stream;

class Solo {
    void lambdas() {
        Consumer<Tagine> f = t -> t.simmer(); Consumer<Cassole> g = t -> t.simmer();
        //                          ^ d: src/main/java/tagine/Tagine.java:8
        //                                                                 ^ d: src/main/java/tagine/Cassole.java:4
        Consumer<Tagine> h = t -> t.simmer(); Stream.of(new Cassole()).forEach(t -> t.simmer());
        //                                                                            ^ d: !jump
        Consumer<Tagine> a = t -> t.simmer(); Consumer<Cassole> b = t -> {
            t.simmer();
            //^ d: !jump
        };
    }
}
