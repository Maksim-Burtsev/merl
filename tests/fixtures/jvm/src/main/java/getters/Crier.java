package getters;

public class Crier {
    private Herald herald;

    void announce(String code) {
        herald.emit("verify", builder -> builder.attribute("code", code)
                .attribute("again", code)
//               ^ d: src/main/java/getters/Notice.java:20
                .attribute("third", code));
        herald.emit(
                "verify",
                builder -> builder.attribute("code", code));
//                                 ^ d: src/main/java/getters/Notice.java:20
        herald.cry("verify", builder -> builder.attribute("code", code));
//                                              ^ d: !jump
    }
}
