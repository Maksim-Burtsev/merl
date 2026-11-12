package chalice;

class Pour {
    Chalice v;
    Noggin m;
    Object o;

    void casts() {
        ((Goblet) v).fill();
        //           ^ d: src/main/java/chalice/Goblet.java:4
        ((Goblet) o).size();
        //           ^ d: src/main/java/chalice/Goblet.java:8
        ((Goblet) (Object) m).fill();
        //                    ^ d: src/main/java/chalice/Goblet.java:4
    }

    void plain() {
        m.fill();
        //^ d: src/main/java/chalice/Noggin.java:4
        v.fill();
        //^ d: src/main/java/chalice/Chalice.java:4
    }

    void around(Chalice x) {
        x.size();
        //^ d: src/main/java/chalice/Chalice.java:6
        if (x instanceof Goblet y) {
            y.size();
            //^ d: src/main/java/chalice/Goblet.java:8
        }
        x.fill();
        //^ d: src/main/java/chalice/Chalice.java:4
    }

    void bare(Chalice x) {
        if (x instanceof Goblet) {
            x.size();
            //^ d: picker src/main/java/chalice/Chalice.java:6, src/main/java/chalice/Goblet.java:8, src/main/java/chalice/Noggin.java:8, src/main/java/ladle/Cup.java:5, src/main/java/ladle/Jar.java:5, src/main/java/ladle/Mug.java:5
        }
    }
}
