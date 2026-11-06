package rebound;

public class Urn {
    public void decant() {
    }

    void anon() { Runnable r = new Ewer() { public void run() { this.decant(); } }; }
    //                                                               ^ d: picker src/main/java/rebound/Ewer.java:4, …

    void after() { this.decant(); }
    //                  ^ d: src/main/java/rebound/Urn.java:4
}
