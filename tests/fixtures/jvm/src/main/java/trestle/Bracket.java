package trestle;

public class Bracket<U> extends Trestle<U> {
    public void tidy() {
        crown().sand();
//              ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
    }
}

class Loose extends Trestle {
    public void tidy() {
        crown().sand();
//              ^ d: picker src/main/java/trestle/Dowel.java:4, src/main/java/trestle/Plank.java:4
    }
}
