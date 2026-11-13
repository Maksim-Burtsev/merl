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
        //     ^ d: picker src/main/java/tagine/Cassole.java:4, src/main/java/tagine/Tagine.java:8, src/main/java/tea/Kettle.java:79; want src/main/java/tagine/Tagine.java:8 (#1514)
    }

    void lambda() {
        Consumer<Tagine> f = Tagine -> Tagine.simmer();
        //                                    ^ d: picker src/main/java/tagine/Cassole.java:4, src/main/java/tagine/Tagine.java:8, src/main/java/tea/Kettle.java:79; want src/main/java/tagine/Tagine.java:8 (#1515)
    }

    class Inner {
        void inner() {
            Tagine.simmer();
            //     ^ d: src/main/java/tagine/Cassole.java:4
        }
    }
}
