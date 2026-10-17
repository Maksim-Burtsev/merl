package tea;

public class Teapot extends Vessel {
    public int pour(int cups) {
        return cups;
    }

    public int pour(int cups, int lid) {
        return cups + lid;
    }

    public int pour(String cups) {
        return cups.length();
    }

    @Override
    public int rinse() {
        return super.rinse() + spout;
        //           ^ d: src/main/java/tea/Vessel.java:6
        //                     ^ d: src/main/java/tea/Vessel.java:4
    }
}
