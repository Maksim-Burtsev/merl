package namesake.b;

import namesake.a.Decanter;

class Cellar {
    static class Decanter {
        void aerate() {}
    }

    Decanter kept = new Decanter();

    void pour(Decanter d) {
        d.aerate();
        //^ d: src/main/java/namesake/b/Cellar.java:7
        kept.aerate();
        //   ^ d: src/main/java/namesake/b/Cellar.java:7
        Decanter c = new Decanter();
        c.aerate();
        //^ d: src/main/java/namesake/b/Cellar.java:7
    }
}

class Rack {
    void hold(Decanter d) {
        d.aerate();
        //^ d: src/main/java/namesake/a/Decanter.java:4
    }
}

class Magnum {
    Decanter bottled = new Decanter();

    Decanter uncork() {
        return bottled;
    }
}

class Jeroboam extends Decanter {
}

class Sommelier {
    static class Decanter {
        void aerate() {}
    }

    void unseal(Magnum s, Jeroboam f) {
        s.bottled.aerate();
        //        ^ d: src/main/java/namesake/a/Decanter.java:4
        s.uncork().aerate();
        //         ^ d: src/main/java/namesake/a/Decanter.java:4
        f.aerate();
        //^ d: src/main/java/namesake/a/Decanter.java:4
    }
}

class Cooper {
    Decanter cellared = new Decanter();

    class Inner {
        class Decanter {
            void aerate() {}
        }

        void swill() {
            cellared.aerate();
            //       ^ d: src/main/java/namesake/a/Decanter.java:4
        }
    }
}
