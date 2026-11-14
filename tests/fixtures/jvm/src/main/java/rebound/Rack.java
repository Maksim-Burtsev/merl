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

    void quotedFor() { use("for (x : es)"); x.decant(); }
    //                                        ^ d: src/main/java/rebound/Flask.java:4

    void declarators() { Ewer e = new Ewer(), x = e; x.decant(); }
    //                                                 ^ d: src/main/java/rebound/Ewer.java:4
    void declaratorName() { Ewer e = new Ewer(), x = e; x.decant(); }
    //                                                  ^ d: src/main/java/rebound/Rack.java:114
    // status: local
    void genericDeclarators() { java.util.Map<String, Ewer> m = new java.util.HashMap<>(), x = m; x.size(); }
    //                                                                                            ^ d: src/main/java/rebound/Rack.java:117
    // status: local
    void callDeclarators() { Ewer e = make(1, 2), x = make(3, 4); x.decant(); }
    //                                                              ^ d: src/main/java/rebound/Ewer.java:4
    void initializerReads() { Ewer e = new Ewer(), y = x; x.decant(); }
    //                                                      ^ d: src/main/java/rebound/Flask.java:4
    void lambdaInitializer() { java.util.function.Function<Ewer, Ewer> f = x -> x, g = null; x.decant(); }
    //                                                                                         ^ d: src/main/java/rebound/Flask.java:4
    void quotedDeclarators() { use("Ewer e = new Ewer(), x = e;"); x.decant(); }
    //                                                               ^ d: src/main/java/rebound/Flask.java:4
    void closedDeclarators() { { Ewer e = new Ewer(), x = e; } x.decant(); }
    //                                                           ^ d: src/main/java/rebound/Flask.java:4

    void declaratorLine() {
        Ewer e = new Ewer(), x = e;
        x.decant();
        //^ d: src/main/java/rebound/Ewer.java:4
    }

    void caseGroups(int k) {
        switch (k) {
            case 1:
                Ewer x = new Ewer();
                break;
            case 2:
                x.decant();
                //^ d: src/main/java/rebound/Ewer.java:4
                break;
        }
        x.decant();
        //^ d: src/main/java/rebound/Flask.java:4
    }

    void caseOneLine(int k) {
        switch (k) {
            case 1: Ewer x = new Ewer(); x.decant(); break;
            //                             ^ d: src/main/java/rebound/Ewer.java:4
            default: x.decant();
            //         ^ d: src/main/java/rebound/Ewer.java:4
        }
    }

    void caseBlock(int k) {
        switch (k) {
            case 1: {
                Ewer x = new Ewer();
                break;
            }
            case 2:
                x.decant();
                //^ d: src/main/java/rebound/Flask.java:4
        }
    }

    void caseNested(int k) {
        switch (k) {
            case 1:
                if (k > 0) {
                    Ewer x = new Ewer();
                }
                break;
            default:
                x.decant();
                //^ d: src/main/java/rebound/Flask.java:4
        }
    }

    void earlierPattern(Object o) {
        switch (o) {
            case Ewer x:
                switch (1) { default: break; }
                x.decant();
                //^ d: src/main/java/rebound/Ewer.java:4
                break;
            default:
                x.decant();
              // ^ d: src/main/java/rebound/Rack.java:6
        }
    }

    void arrowPattern(Object o) {
        switch (o) {
            case Ewer x -> {
                switch (1) { default -> { } }
                x.decant();
                //^ d: src/main/java/rebound/Ewer.java:4
            }
            default -> { }
        }
    }

    void forInit() { for (Ewer d = new Ewer(), x = d; ; ) { x.decant(); } }
    //                                                      ^ d: src/main/java/rebound/Rack.java:209
    //                                                        ^ d: src/main/java/rebound/Ewer.java:4

    void forInitClosed() { for (Ewer d = new Ewer(), x = d; ; ) { } x.decant(); }
    //                                                              ^ d: src/main/java/rebound/Rack.java:6

    void forInitAbove() {
        for (Ewer d = new Ewer(), x = d; ; ) {
            x.decant();
            //^ d: src/main/java/rebound/Ewer.java:4
        }
    }

    void continued() {
        Ewer d = new Ewer(),
            x = d;
        x.decant();
      //^ d: src/main/java/rebound/Rack.java:225
    }

    void continuedRead() {
        Ewer d = make(1,
            2), e = x;
        x.decant();
      //^ d: src/main/java/rebound/Rack.java:6
    }

    Ewer make(int a, int b) {
        return null;
    }

    void use(String s) {
    }

    void bareDeclarators() { int a, x; x = 1; use("" + x); }
    //                                                 ^ d: src/main/java/rebound/Rack.java:244

    void threeBareDeclarators() { int a, b, x; x = 1; use("" + x); }
    //                                                         ^ d: src/main/java/rebound/Rack.java:247
}
