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
        //           ^ d: picker src/main/java/tea/Teapot.java:17, src/main/java/tea/Kettle.java:44, src/main/java/tea/Vessel.java:6; want src/main/java/tea/Vessel.java:6 (#963)
        //                     ^ d: src/main/java/tea/Vessel.java:4
    }
}
