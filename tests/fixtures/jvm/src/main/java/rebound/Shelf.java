package rebound;

public class Shelf {
    private Flask x = new Flask();

    void local() { Ewer x = new Ewer(); x.decant(); }
    //                                    ^ d: picker src/main/java/rebound/Ewer.java:4, …

    void varLocal() { var x = new Ewer(); x.decant(); }
    //                                      ^ d: picker src/main/java/rebound/Ewer.java:4, …

    void useThenDeclare() { x.decant(); Ewer x = new Ewer(); }
    //                        ^ d: src/main/java/rebound/Flask.java:4

    void closedBlock() { { Ewer x = new Ewer(); } x.decant(); }
    //                                              ^ d: src/main/java/rebound/Flask.java:4

    void negated(Object o) {
        if (!(o instanceof Ewer x)) {
            return;
        }
        x.decant();
        //^ d: src/main/java/rebound/Ewer.java:4
    }

    void negatedNoExit(Object o) {
        if (!(o instanceof Ewer x)) {
            run();
        }
        x.decant();
        //^ d: src/main/java/rebound/Flask.java:4
    }

    void arrows(Object o) {
        switch (o) {
            case Ewer x -> x.decant();
            //               ^ d: src/main/java/rebound/Ewer.java:4
            case Spill x -> x.decant();
            //                ^ d: src/main/java/rebound/Spill.java:4
            default -> { }
        }
    }

    void colons(Object o) {
        switch (o) {
            case Ewer x:
                x.decant();
                //^ d: src/main/java/rebound/Ewer.java:4
                break;
            case Spill x:
                x.decant();
                //^ d: src/main/java/rebound/Spill.java:4
                break;
            default:
                break;
        }
    }

    void anonymous() {
        Flask z = new Flask();
        java.util.function.Consumer<Ewer> c = new java.util.function.Consumer<Ewer>() { public void accept(Ewer z) { z.decant(); } };
        //                                                                                                             ^ d: picker src/main/java/rebound/Ewer.java:4, …
        z.decant();
        //^ d: src/main/java/rebound/Flask.java:4
    }

    void run() {
    }
}
