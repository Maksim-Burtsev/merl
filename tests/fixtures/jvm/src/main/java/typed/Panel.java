package typed;

import java.util.ArrayList;
import java.util.function.IntSupplier;

class Panel {
    private final Meter main = new Meter();

    int sum(Meter meter) {
        return meter.reading();
        //           ^ d: src/main/java/typed/Meter.java:4
        // status: reading → Meter.reading (via meter: Meter)
    }

    void stash(String name) {}

    void fill() {
        var names = new ArrayList<String>();
        names.stash("a");
        //    ^ d: none
        var gauge = new Gauge();
        gauge.reading();
        //    ^ d: src/main/java/typed/Gauge.java:4
        IntSupplier read = main::reading;
        //                       ^ d: src/main/java/typed/Meter.java:4
        this.main.reading();
        //        ^ d: src/main/java/typed/Meter.java:4
        // status: via this.main: Meter
    }

    boolean yes() {
        return "true".equals("x");
        //            ^ d: none
    }

    <T extends Gauge> int pick(T item) {
        return item.reading();
        //          ^ d: picker src/main/java/typed/Gauge.java:4, src/main/java/typed/Meter.java:4
    }

    int hop(Meter meter) {
        var twin = meter.twin();
        return twin.reading();
        //          ^ d: src/main/java/typed/Gauge.java:4
        // status: via twin: Gauge
    }
}
