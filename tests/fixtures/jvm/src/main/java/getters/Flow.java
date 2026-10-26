package getters;

import java.util.List;
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
}
