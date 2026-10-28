package getters;

import java.util.Optional;
import reactor.core.publisher.Mono;

public class Crier {
    private Herald herald;

    void announce(String code, Account account, Spec spec) {
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
        Mono.just(Ticket.from(account))
                .flatMap(ticket -> Optional.of(spec).map(s -> {
                    Optional.of(s).ifPresent(ticket::setSpec);
//                                                    ^ d: src/main/java/getters/Ticket.java:13
                    return ticket;
                }).map(Mono::just).orElse(Mono.empty()));
        Mono.just(Ticket.of(account)).doOnNext(ticket -> Optional.of(spec).ifPresent(ticket::setSpec));
//                                                                                           ^ d: !jump
    }
}
