package gravy;

// A field, a local and a parameter named like the class Gravy are variables of type Boat
// (#1515).
public class Pour {
    Boat Gravy = new Boat();

    void field() {
        Gravy.drizzle();
        //    ^ d: src/main/java/gravy/Boat.java:4
    }

    void local() {
        var Gravy = new Boat();
        Gravy.drizzle();
        //    ^ d: src/main/java/gravy/Boat.java:4
    }

    void param(Boat Gravy) {
        Gravy.drizzle();
        //    ^ d: src/main/java/gravy/Boat.java:4
    }
}
