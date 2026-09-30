package accessors.c;

import accessors.a.Frozen;
import accessors.a.LeafSpec;
import accessors.a.Poster;
import accessors.a.Stamp;
import java.util.function.Function;

class Wall {
    Poster load() {
        return null;
    }

    String read(Poster poster, Stamp stamp, Frozen frozen) {
        String caption = load().getCaption();
        //                      ^ d: picker src/main/java/accessors/a/Poster.java:9
        // status: getCaption: by name, 1 match
        String same = poster.getCaption();
        //                   ^ d: src/main/java/accessors/a/Poster.java:9
        // status: getCaption → Poster.caption (via poster: Poster)
        Function<Poster, String> get = Poster::getCaption;
        //                                     ^ d: src/main/java/accessors/a/Poster.java:9
        // status: getCaption → Poster.caption (via Poster)
        int serial = poster.getSerial();
        //                  ^ d: none
        boolean pinned = poster.isPinned();
        //                      ^ d: src/main/java/accessors/a/Poster.java:11
        String secret = poster.getSecret();
        //                     ^ d: none
        String label = stamp.getLabel();
        //                   ^ d: src/main/java/accessors/a/Stamp.java:9
        frozen.setNote("x");
        //     ^ d: none
        Function<LeafSpec, Long> made = LeafSpec::getMade;
        //                                        ^ d: src/main/java/accessors/a/BaseSpec.java:7
        return caption + same + get + serial + pinned + secret + label + made;
    }
}
