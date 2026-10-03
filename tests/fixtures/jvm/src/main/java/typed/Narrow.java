package typed;

import java.util.Map;

class Narrow {
    int cast(Object any) {
        var meter = (Meter) any;
        return meter.reading();
        //           ^ d: src/main/java/typed/Meter.java:4
        // status: via meter: Meter
    }

    int inline(Object any) {
        return ((Gauge) any).reading();
        //                   ^ d: src/main/java/typed/Gauge.java:4
        // status: via Gauge
    }

    int pattern(Object any) {
        if (any instanceof Gauge gauge && gauge.reading() > 0) {
            //                                  ^ d: src/main/java/typed/Gauge.java:4
            return gauge.reading();
            //           ^ d: src/main/java/typed/Gauge.java:4
            // status: via gauge: Gauge
        }
        return 0;
    }

    int call(Object any) {
        return wrap((Meter) any).reading();
        //                       ^ d: picker src/main/java/typed/Gauge.java:4, src/main/java/typed/Meter.java:4
    }

    Object wrap(Object any) {
        return any;
    }

    int dotted(Meter.Dial dial, Map.Entry<String, Gauge> entry) {
        entry.tick();
        //    ^ d: none
        return dial.tick();
        //          ^ d: src/main/java/typed/Meter.java:13
        // status: via dial: Meter.Dial
    }
}
