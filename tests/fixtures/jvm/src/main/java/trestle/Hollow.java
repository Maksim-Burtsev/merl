package trestle;

public class Hollow extends Trestle<Dowel> {
    public void tidy() {
        Object a = new Trestle<Plank>() { // its crown() is a Plank
            public void rasp() {
                crown().sand();
//                      ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
            }
        };
        Object b = new Trestle<Plank>()
        {
            public void rasp() {
                crown().sand();
//                      ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
            }
        };
        Object c = new Trestle<Plank>(
        ) {
            public void rasp() {
                crown().sand();
//                      ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
            }
        };
        Object e = new Trestle<Plank>() { public void rasp() { crown().sand(); } };
//                                                                     ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
        crown().sand();
//              ^ d: src/main/java/trestle/Dowel.java:4
    }
}
