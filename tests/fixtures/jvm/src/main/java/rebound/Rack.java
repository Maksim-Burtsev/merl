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

    void caught() { try { } catch (Ewer x) { } x.decant(); }
    //                                           ^ d: src/main/java/rebound/Flask.java:4

    void bareName(List<Ewer> es) { for (Ewer x : es) { } x.decant(); }
    //                                                   ^ d: src/main/java/rebound/Rack.java:6

    void elseBlockReturns(Object o) { if (o instanceof Ewer x) { } else { return; } x.decant(); }
    //                                                                                ^ d: src/main/java/rebound/Ewer.java:4
    //                                                                              ^ d: src/main/java/rebound/Rack.java:53

    void elseReturns(Object o) { if (o instanceof Ewer x) { } else return; x.decant(); }
    //                                                                       ^ d: src/main/java/rebound/Ewer.java:4

    void elseThrows(Object o) { if (o instanceof Ewer x) { } else throw new IllegalStateException(); x.decant(); }
    //                                                                                                 ^ d: src/main/java/rebound/Ewer.java:4

    void equalsFalse(Object o) { if (o instanceof Ewer x == false) return; x.decant(); }
    //                                                                       ^ d: src/main/java/rebound/Ewer.java:4

    void inElse(Object o) { if (o instanceof Ewer x) { } else { x.decant(); } }
    //                                                            ^ d: src/main/java/rebound/Flask.java:4

    void recordThenIf(Object o, Object q) { if (o instanceof Box(int y)) { } if (q instanceof Ewer x) { x.decant(); } }
    //                                                                                                    ^ d: src/main/java/rebound/Ewer.java:4
    //                                                                                                  ^ d: src/main/java/rebound/Rack.java:69

    void recordThenStatement(Object o, Object q) { if (o instanceof Box(int y)) { } if (q instanceof Ewer x) x.decant(); }
    //                                                                                                         ^ d: src/main/java/rebound/Ewer.java:4

    void recordThenCatch(Object o) { if (o instanceof Box(int y)) { } try { } catch (Ewer x) { x.decant(); } }
    //                                                                                           ^ d: src/main/java/rebound/Ewer.java:4

    void recordThenRecord(Object o, Object q) { if (o instanceof Box(int y)) { } if (q instanceof Box(Ewer x)) { x.decant(); } }
    //                                                                                                             ^ d: src/main/java/rebound/Ewer.java:4

    void catchInsideRecord(Object o) { if (o instanceof Box(int y)) { try { } catch (Ewer x) { } x.decant(); } }
    //                                                                                             ^ d: src/main/java/rebound/Flask.java:4

    void commentAfterBlock(List<Ewer> es) { for (Ewer x : es) { } /* } */ x.decant(); }
    //                                                                      ^ d: src/main/java/rebound/Flask.java:4

    void declaredPattern(Object o) { boolean b = o instanceof Ewer x; x.decant(); }
    //                                                                  ^ d: src/main/java/rebound/Flask.java:4

    void argumentPattern(Object o) { use(o instanceof Ewer x ? "" : ""); x.decant(); }
    //                                                                     ^ d: src/main/java/rebound/Flask.java:4

    void whilePattern(Object o) { while (!(o instanceof Ewer x)) { o = null; } x.decant(); }
    //                                                                           ^ d: src/main/java/rebound/Ewer.java:4

    void elseFinishes(Object o) { if (o instanceof Ewer x) { } else { use("return"); } x.decant(); }
    //                                                                                   ^ d: src/main/java/rebound/Flask.java:4

    void elseLoops(Object o) { if (o instanceof Ewer x) { } else { while (true) { } } x.decant(); }
    //                                                                                  ^ d: src/main/java/rebound/Ewer.java:4

    void caseArrow(Object o) { switch (o) { case Ewer x -> { } default -> { } } x.decant(); }
    //                                                                            ^ d: src/main/java/rebound/Flask.java:4

    void caseColon(Object o) { switch (o) { case Ewer x: break; default: x.decant(); } }
    //                                                                     ^ d: src/main/java/rebound/Flask.java:4

    void use(String s) {
    }
}
