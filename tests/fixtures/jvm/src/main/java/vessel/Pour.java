package vessel;

class Pour {
    Vessel v;
    Mug m;
    Object o;

    void casts() {
        ((Cup) v).fill();
        //        ^ d: src/main/java/vessel/Cup.java:4
        ((Cup) o).size();
        //        ^ d: src/main/java/vessel/Cup.java:8
        ((Cup) (Object) m).fill();
        //                 ^ d: src/main/java/vessel/Cup.java:4
    }

    void plain() {
        m.fill();
        //^ d: src/main/java/vessel/Mug.java:4
        v.fill();
        //^ d: src/main/java/vessel/Vessel.java:4
    }

    void around(Vessel x) {
        x.size();
        //^ d: src/main/java/vessel/Vessel.java:6
        if (x instanceof Cup y) {
            y.size();
            //^ d: src/main/java/vessel/Cup.java:8
        }
        x.fill();
        //^ d: src/main/java/vessel/Vessel.java:4
    }

    void bare(Vessel x) {
        if (x instanceof Cup) {
            x.size();
            //^ d: picker src/main/java/vessel/Cup.java:8, src/main/java/vessel/Mug.java:8, src/main/java/vessel/Vessel.java:6
        }
    }
}
