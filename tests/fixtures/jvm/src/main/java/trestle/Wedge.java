package trestle;

public class Wedge extends Trestle<Plank, Dowel> {
    public void tidy() {
        crown().sand();
//              ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
    }
}
