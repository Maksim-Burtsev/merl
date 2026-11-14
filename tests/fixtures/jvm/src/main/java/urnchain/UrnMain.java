package urnchain;

public class UrnMain {
    static class Pot {
        void boil() {
        }
    }

    void run() {
        Urn.builder().size(2).build().boil();
        //  ^ d: src/main/java/urnchain/Urn.java:11
        //            ^ d: src/main/java/urnchain/UrnBuilder.java:4
        //                    ^ d: src/main/java/urnchain/UrnBuilder.java:8
        //                            ^ d: src/main/java/urnchain/Urn.java:4
        Urn.builder()
            .size(2)
            //^ d: src/main/java/urnchain/UrnBuilder.java:4
            .build()
            //^ d: src/main/java/urnchain/UrnBuilder.java:8
            .boil();
            //^ d: src/main/java/urnchain/Urn.java:4
        Mystery.builder().size(2).boil();
        //                ^ d: !jump
        //                        ^ d: !jump
    }

    void stockLocals() {
        var b = Urn.builder();
        b.size(2);
        //^ d: src/main/java/urnchain/UrnBuilder.java:4
        var u = Urn.builder().size(2).build();
        u.boil();
        //^ d: src/main/java/urnchain/Urn.java:4
        var w = Urn.builder()
            .size(2);
        w.build();
        //^ d: src/main/java/urnchain/UrnBuilder.java:8
        var m = Mystery.builder();
        m.size(2);
        //^ d: !jump
        var s = Urn.builder().size(2).build().mystery();
        s.boil();
        //^ d: !jump
    }
}
