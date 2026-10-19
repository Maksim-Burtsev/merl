package tea;

public class Table {
    public int set(Teapot pot, Kettle kettle) {
        return pot.pour(1) + pot.pour(1, 2) + pot.pour("two") + kettle.pour(3);
        //         ^ d: src/main/java/tea/Teapot.java:4
        //                       ^ d: src/main/java/tea/Teapot.java:8
        //                                        ^ d: src/main/java/tea/Teapot.java:12
        //                                                             ^ d: src/main/java/tea/Kettle.java:22
    }

    public int cover() {
        Lid lid = new Lid();
        //            ^ d: src/main/java/tea/Lid.java:3
        return lid.heft() + new Kettle.Lid().pour(2);
        //         ^ d: src/main/java/tea/Lid.java:4
        //                             ^ d: src/main/java/tea/Kettle.java:52
        //                                   ^ d: src/main/java/tea/Kettle.java:53
    }

    public int old(tea.stale.Kettle stale, Teapot pot) {
        return stale.pour(1) + new Kettle(2).serve() + pot.rinse();
        //           ^ d: src/main/java/tea/stale/Kettle.java:4
        //                         ^ d: src/main/java/tea/Kettle.java:16
        //                                   ^ d: src/main/java/tea/Kettle.java:32
        //                                                 ^ d: src/main/java/tea/Teapot.java:17
    }

    public Teapot spare() {
        //     ^ d: src/main/java/tea/Teapot.java:3
        return new Teapot();
    }

    public int chained(Kettle kettle) {
        return kettle
            .pour(3);
        //   ^ d: src/main/java/tea/Kettle.java:22
    }
}
