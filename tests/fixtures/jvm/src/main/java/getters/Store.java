package getters;

import reactor.core.publisher.Flux;
import reactor.core.publisher.Mono;

public interface Store {
    <E extends Account> Mono<E> fetch(Class<E> type, String name);

    Mono<Owner> fetch(String name);

    <E> Mono<Owner> raw(Class<E> type);

    Flux<Account> all();

    Flux<Owner> owners();

    Flux<Account> pages();

    Flux<Owner> pages(int size);

    Ticket ticketFor(String name);
}
