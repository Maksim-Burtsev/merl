package rebound;

import java.util.List;

public class Rack {
    private Flask x = new Flask();

    void lambda(List<Ewer> es) { es.forEach(x -> x.decant()); x.decant(); }
    //                                                          ^ d: src/main/java/rebound/Flask.java:4

    void forBlock(List<Ewer> es) { for (Ewer x : es) { } x.decant(); }
    //                                                     ^ d: src/main/java/rebound/Flask.java:4

    void forStatement(List<Ewer> es) { for (Ewer x : es) x.decant(); x.decant(); }
    //                                                                 ^ d: src/main/java/rebound/Flask.java:4

    void matched(Object o) { if (o instanceof Ewer x) { } x.decant(); }
    //                                                      ^ d: src/main/java/rebound/Flask.java:4

    void resource() { try (Ewer x = new Ewer()) { } x.decant(); }
    //                                                ^ d: src/main/java/rebound/Flask.java:4

    void forBody(List<Ewer> es) { for (Ewer x : es) { x.decant(); } }
    //                                                  ^ d: src/main/java/rebound/Ewer.java:4

    void forElse(List<Ewer> es) { for (Ewer x : es) if (es.isEmpty()) x.decant(); else x.decant(); }
    //                                                                                   ^ d: src/main/java/rebound/Ewer.java:4

    void negatedLine(Object o) { if (!(o instanceof Ewer x)) return; x.decant(); }
    //                                                                 ^ d: src/main/java/rebound/Ewer.java:4

    void braceInComment(List<Ewer> es) { for (Ewer x : es) /* } */ x.decant(); }
    //                                                               ^ d: src/main/java/rebound/Ewer.java:4

    void braceInString(List<Ewer> es) { for (Ewer x : es) use("}"); x.decant(); }
    //                                                                ^ d: src/main/java/rebound/Flask.java:4

    void plain() { x.decant(); }
    //               ^ d: src/main/java/rebound/Flask.java:4

    void later(List<Ewer> es) {
        for (Ewer x : es) { }
        x.decant();
        //^ d: src/main/java/rebound/Flask.java:4
    }

    void use(String s) {
    }
}
