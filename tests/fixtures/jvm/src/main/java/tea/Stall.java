package tea;

import tea.stale.Kettle;

public class Stall {
    public int sell() {
        Kettle old = new Kettle();
        //               ^ d: src/main/java/tea/stale/Kettle.java:3
        return old.pour(1);
        //         ^ d: src/main/java/tea/stale/Kettle.java:4
    }
}
