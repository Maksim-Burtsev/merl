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
