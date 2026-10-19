package arity;

import java.util.List;

public class Visit {
    void walk(List<String> people) {
        people.stream().filter(p -> p.isEmpty()).map(p -> p.usher(p)).count();
//                                                          ^ d: picker src/main/java/arity/Door.java:4, src/main/java/arity/Gate.java:4
        people.forEach(p -> p.usher(p, 1));
//                            ^ d: picker src/main/java/arity/Gate.java:4, src/main/java/arity/Door.java:4
        people.forEach(p -> p.unbar(p));
//                            ^ d: src/main/java/arity/Door.java:8
        people.forEach(p -> p.latch(p, p, p));
//                            ^ d: src/main/java/arity/Hatch.java:8
        LoudKt.holler("hey");
//             ^ d: src/main/kotlin/arity/Loud.kt:3
    }

    void later() {
        var p = make();
        String.join("""
            a
            """, p.unbar("x"));
//                 ^ d: src/main/java/arity/Door.java:8
    }

    String note() {
        return String.format("""
            %s, %s
            """, "a", "b");
    }
}
