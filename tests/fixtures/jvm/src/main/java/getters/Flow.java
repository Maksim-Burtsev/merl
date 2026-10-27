package getters;

import java.util.List;
import java.util.stream.Collectors;
import reactor.core.publisher.Mono;

public class Flow {
    private Store store;

    void show(List<Account> accounts, Mono<Account> one) {
        store.fetch(Account.class, "a").map(account -> account.getSpec().getDisplayName());
//                                                                        ^ d: src/main/java/getters/Spec.java:7
        store.fetch(Account.class, "a")
                .filter(a -> a != null)
                .switchIfEmpty(Mono.error(() -> new IllegalStateException("none (yet)")))
                .flatMap(account -> {
                    account.getSpec().getOwner();
//                                    ^ d: src/main/java/getters/Spec.java:8
                    return Mono.empty();
                });
        store.all().doOnNext(account -> account.getSpec().getDisplayName());
//                                                         ^ d: src/main/java/getters/Spec.java:7
        accounts.stream().filter(account -> account.getSpec().getDisplayName() != null);
//                                                            ^ d: src/main/java/getters/Spec.java:7
        one.map(account -> account.getSpec().getDisplayName());
//                                           ^ d: src/main/java/getters/Spec.java:7
        store.owners().map(owner -> owner.getDisplayName());
//                                        ^ d: src/main/java/getters/Owner.java:7
        store.all().map(Account::getSpec).map(account -> account.getSpec().getDisplayName());
//                                                                          ^ d: !jump
        store.pages().map(account   -> account.getSpec().getDisplayName());
//                                                        ^ d: !jump
        store.raw(Account.class).map(account -> account.getSpec().getDisplayName());
//                                                                 ^ d: !jump
        store.fetch(Account.class, "a").map((a, b) -> a.getSpec().getDisplayName());
//                                                                 ^ d: !jump
        store.fetch(Account.class, "a").map(account -> account.getSpec()).map(account -> account.getSpec().getDisplayName());
//                                                                                                          ^ d: !jump
    }

    void tickets(Ticket ticket, Draft draft) {
        ticket.getSpec().getDisplayName();
//             ^ d: src/main/java/getters/Ticket.java:13
        draft.getSpec();
//            ^ d: !jump
        draft.getCopy();
//            ^ d: !jump
    }

    void references(List<Account> accounts) {
        accounts.stream().map(Account::getSpec).map(spec -> spec.getDisplayName());
//                                                               ^ d: src/main/java/getters/Spec.java:7
        accounts.stream().map(Ticket::from).collect(Collectors.toMap(ticket -> ticket.getSpec(), t -> t));
//                                                                                    ^ d: src/main/java/getters/Ticket.java:13
        accounts.stream().map(Ticket::of).map(ticket -> ticket.getSpec());
//                                                             ^ d: !jump
        accounts.stream().map(Ticket::label).map(ticket -> ticket.getSpec());
//                                                                ^ d: !jump
        store.all().filter(a -> a != null).next().map(account -> account.getSpec().getDisplayName());
//                                                                                 ^ d: src/main/java/getters/Spec.java:7
    }

    void assigned(Account account) {
        var issued = store.ticketFor(account.getSpec().getDisplayName());
        issued.getSpec();
//             ^ d: src/main/java/getters/Ticket.java:13
    }
}
