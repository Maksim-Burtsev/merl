package typeargs;

import java.util.Map;

public class Build {
    String index() {
        return Specs.<Build, Map<String, ?>>single("spec.name", Map.class);
//                                          ^ d: src/main/java/typeargs/Specs.java:4
    }
}
