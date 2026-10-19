package quern;

import static quern.Knobs.*;

import java.util.ArrayList;
import java.util.List;
import quern.parts.*;

/* public abstract class Stone {
 *     public int turn() { return 0; }
 * } */
public class Quern extends Stone implements Turnable {
    private Hopper hopper = new Hopper();
    private final List<Stone> stones = new ArrayList<>();
    private int burr = 3;

    @Override
    public int turn() {
        // public int wear() {
        return super.turn() + wear() + crank() + Turnable.super.crank() + notch();
        //           ^ d: src/main/java/quern/Stone.java:4
        //                    ^ d: src/main/java/quern/Stone.java:8
        //                             ^ d: src/main/java/quern/Turnable.java:4
        //                                                      ^ d: src/main/java/quern/Turnable.java:4
        //                                                                ^ d: src/main/java/quern/Knobs.java:4
    }

    public Hopper hopper() {
        return hopper;
    }

    public String label(Spout spout) {
        StringBuilder sb = new StringBuilder();
        sb.append("public Spout append(String s) {");
        // ^ d: none
        String name = " q ";
        return name.strip() + spout.append("y").strip();
        //          ^ d: none
        //                          ^ d: src/main/java/quern/Spout.java:4
        //                                      ^ d: src/main/java/quern/Spout.java:8
    }

    public int mix() {
        Grist g = Grist.builder()
            .coarse(2)
        //   ^ d: src/main/java/quern/Grist.java:21
            .weight(3)
        //   ^ d: src/main/java/quern/Grist.java:25
            .build();
        //   ^ d: src/main/java/quern/Grist.java:30
        return g.weight() + Grist.builder().coarse(1).build().weight();
        //       ^ d: src/main/java/quern/Grist.java:10
        //                                  ^ d: src/main/java/quern/Grist.java:21
        //                                                    ^ d: src/main/java/quern/Grist.java:10
    }

    public int shadow(Object o) {
        Hopper hopper = hopper();
        //              ^ d: src/main/java/quern/Quern.java:28
        hopper = hopper.next();
        //       ^ d: src/main/java/quern/Quern.java:58
        //              ^ d: src/main/java/quern/Hopper.java:8
        int total = hopper.charge() + this.hopper.charge() + new Runner().whirl();
        //          ^ d: src/main/java/quern/Quern.java:58
        //                                 ^ d: src/main/java/quern/Quern.java:13
        //                                                       ^ d: src/main/java/quern/parts/Runner.java:3
        //                                                                ^ d: src/main/java/quern/parts/Runner.java:4
        //                 ^ d: src/main/java/quern/Hopper.java:4
        //                                        ^ d: src/main/java/quern/Hopper.java:4
        for (Stone burr : stones) {
            total += burr.wear() + this.burr;
            //       ^ d: src/main/java/quern/Quern.java:70
            //            ^ d: src/main/java/quern/Stone.java:8
            //                          ^ d: src/main/java/quern/Quern.java:15
        }
        total += stones.get(0).turn();
        //                     ^ d: picker src/main/java/quern/Quern.java:18, src/main/java/quern/Stone.java:4; want src/main/java/quern/Stone.java:4 (#1129)
        if (o instanceof HandMill mill) {
            total += mill.crank();
            //       ^ d: src/main/java/quern/Quern.java:78
            //            ^ d: src/main/java/quern/HandMill.java:5
        }
        if (o instanceof Notch(int depth)) {
            total += depth;
            //       ^ d: src/main/java/quern/Quern.java:83
        }
        return switch (o) {
            case Notch(int depth) when depth > 1 -> total + depth;
            //                                              ^ d: src/main/java/quern/Quern.java:88
            default -> total;
        };
    }

    public int scopes(Notch notch) {
        Turnable t = new Turnable() {
            @Override
            public int crank() {
                return burr + this.hashCode();
                //     ^ d: src/main/java/quern/Quern.java:15
            }
        };
        return t.crank() + notch.depth() + notch() + new Axle().grip();
        //       ^ d: src/main/java/quern/Turnable.java:4
        //                       ^ d: src/main/java/quern/Notch.java:3
        //                                 ^ d: src/main/java/quern/Knobs.java:4
        //                                                      ^ d: src/main/java/quern/Quern.java:112
    }

    class Axle {
        private int burr = 9;

        int grip() {
            return burr + Quern.this.burr + wear();
            //     ^ d: src/main/java/quern/Quern.java:110
            //                       ^ d: src/main/java/quern/Quern.java:15
            //                              ^ d: src/main/java/quern/Stone.java:8
        }
    }
}
