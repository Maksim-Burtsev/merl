package pins;

class Uses {
    void lamp(Lamp l) {
        l.glow();
        //^ d: src/main/java/pins/Lamp.java:4
    }

    void fuse(Fuse f, Wick w, Case.Latch latch) {
        f.blow();
        //^ d: src/main/java/pins/Fuse.java:7
        w.burn();
        //^ d: picker src/main/java/pins/Wick.java:4, src/main/java/pins/Spare.java:6
        latch.hook();
        //    ^ d: src/main/java/pins/Case.java:7
    }

    void held(Holder holder) {
        var f = holder.getFuse();
        f.blow();
        //^ d: src/main/java/pins/Fuse.java:7
        java.util.function.Supplier<Object> s = Optional::getFuse;
        //                                                ^ d: none
    }
}
