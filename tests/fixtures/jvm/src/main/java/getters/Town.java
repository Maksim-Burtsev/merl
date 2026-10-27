package getters;

import java.util.function.Consumer;

public class Town {
    private Herald herald;

    void emit(String reason, Consumer<Gazette> gazette) {
    }

    void call(String code) {
        herald.<Notice>emit("verify", builder -> builder.attribute("code", code));
//                                                       ^ d: src/main/java/getters/Notice.java:20
        emit("verify", gazette -> gazette.attribute("code", code));
//                                        ^ d: src/main/java/getters/Gazette.java:4
    }
}
