package trestle;

public class Yard {
    public static void main(String[] a) {
        Plinth p = new Plinth();
        p.crown().sand();
//                ^ d: src/main/java/trestle/Dowel.java:4
        Cradle c = new Cradle();
        c.crown().sand();
//                ^ d: src/main/java/trestle/Dowel.java:4
        p.grab(new Dowel()).sand();
//                          ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
    }
}
