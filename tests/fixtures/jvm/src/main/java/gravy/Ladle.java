package gravy;

// Pour declares a field and a local named Gravy: they stay its own and leave the class Gravy
// whole in this file (#1514).
public class Ladle extends Gravy {
    @Override
    public void drizzle() {
        super.drizzle();
        //    ^ d: src/main/java/gravy/Gravy.java:8
    }

    void chain() {
        Gravy.make().drizzle();
        //           ^ d: src/main/java/gravy/Gravy.java:8
        Gravy g = new Gravy();
        g.drizzle();
        //^ d: src/main/java/gravy/Gravy.java:8
    }
}
