package imports.c;

import imports.a.Member;
//     ^ d: none
//               ^ d: src/main/java/imports/a/Member.java:3
import java.util.Objects;
//               ^ d: none
import static imports.d.Checks.*;

class Visit {
    String greet(Member member, String note) {
    //           ^ d: src/main/java/imports/a/Member.java:3
    // status: Member: via import src/main/java/imports/a/Member.java
        if (isVacant(note) || Objects.requireNonNull(note).isEmpty()) {
        //  ^ d: src/main/java/imports/d/Checks.java:4
        //                            ^ d: none
            return "";
        }
        return new Same().toString();
        //         ^ d: src/main/java/imports/c/Same.java:3
    }
}
