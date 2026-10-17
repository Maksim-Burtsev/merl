package tea;

import static tea.Spices.grind;
import static tea.Spices.steam;

import java.util.function.IntUnaryOperator;

/**
 * public class Teapot {
 * public int pour(int cups) { return 0; }
 * Pours like {@link Teapot#pour(int)}.
 */
public class Kettle extends Vessel {
    private int spout;

    public Kettle(int spout) {
        this.spout = spout;
        //   ^ d: src/main/java/tea/Kettle.java:14
        //           ^ d: src/main/java/tea/Kettle.java:16
    }

    public int pour(int cups) {
        return cups + spout;
        //     ^ d: src/main/java/tea/Kettle.java:22
        //            ^ d: src/main/java/tea/Kettle.java:14
    }

    public int grind() {
        return 7;
    }

    public int serve() {
        int spout = 1;
        return spout + this.spout + super.spout + pour(2) + grind() + steam();
        //     ^ d: src/main/java/tea/Kettle.java:33
        //                  ^ d: src/main/java/tea/Kettle.java:14
        //                                ^ d: src/main/java/tea/Vessel.java:4
        //                                        ^ d: src/main/java/tea/Kettle.java:22
        //                                                  ^ d: src/main/java/tea/Kettle.java:28
        //                                                            ^ d: src/main/java/tea/Spices.java:8
    }

    @Override
    public int rinse() {
        return super.rinse() + Spices.grind() + new Lid().pour(1);
        //           ^ d: src/main/java/tea/Vessel.java:6
        //                            ^ d: src/main/java/tea/Spices.java:4
        //                                          ^ d: picker src/main/java/tea/Kettle.java:52, src/main/java/tea/Lid.java:3; want src/main/java/tea/Kettle.java:52 (#965)
        //                                                ^ d: picker src/main/java/tea/Kettle.java:22, src/main/java/tea/Kettle.java:53, src/main/java/tea/Teapot.java:4, src/main/java/tea/Teapot.java:12, src/main/java/tea/stale/Kettle.java:4; want src/main/java/tea/Kettle.java:53 (#965)
    }

    static class Lid {
        int pour(int cups) {
            return cups;
        }
    }

    public int steep() {
        IntUnaryOperator f = spout -> spout + 1;
        //                            ^ d: src/main/java/tea/Kettle.java:59
        return f.applyAsInt(this.spout) + grind();
        //                       ^ d: src/main/java/tea/Kettle.java:14
        //                                ^ d: src/main/java/tea/Kettle.java:28
    }

    public String recipe() {
        return """
            public class Teapot {
                public int pour(int cups) { return 0; }
            }
            """;
    }
}
