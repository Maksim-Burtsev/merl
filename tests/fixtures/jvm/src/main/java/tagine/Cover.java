package tagine;

import java.util.function.Consumer;

// Fireside, in another file, declares a field Tagine of type Cassole: in a subclass the inherited
// field obscures the class Tagine (#1515).
public class Cover extends Fireside {
    void inherited() {
        Tagine.simmer();
        //     ^ d: src/main/java/tagine/Cassole.java:4
    }

    void param(Tagine Tagine) {
        Tagine.simmer();
        //     ^ d: src/main/java/tagine/Tagine.java:8
    }

    void lambda() {
        Consumer<Tagine> f = Tagine -> Tagine.simmer();
        //                                    ^ d: src/main/java/tagine/Tagine.java:8
        Consumer<Cassole> g = Tagine -> Tagine.simmer();
        //                                     ^ d: src/main/java/tagine/Cassole.java:4
        Consumer<Cassole> h = (c) -> c.simmer();
        //                             ^ d: src/main/java/tagine/Cassole.java:4
        Consumer<Tagine> k = Tagine -> Tagine.simmer(); Consumer<Cassole> m = Tagine -> Tagine.simmer();
        //                                    ^ d: src/main/java/tagine/Tagine.java:8
        //                                                                                     ^ d: src/main/java/tagine/Cassole.java:4
        Consumer<Cassole> p = Tagine -> Tagine.simmer(); Consumer<Tagine> q = Tagine -> Tagine.simmer();
        //                                     ^ d: src/main/java/tagine/Cassole.java:4
        //                                                                                     ^ d: src/main/java/tagine/Tagine.java:8
    }

    class Inner {
        void inner() {
            Tagine.simmer();
            //     ^ d: src/main/java/tagine/Cassole.java:4
        }
    }
}
