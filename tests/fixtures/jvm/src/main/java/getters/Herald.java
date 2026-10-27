package getters;

import java.util.function.Consumer;
import reactor.core.publisher.Mono;

public interface Herald {
    Mono<Void> emit(String reason, Consumer<Notice.NoticeBuilder> notice);

    Mono<Void> cry(String reason, Consumer<Notice.NoticeBuilder> notice);

    Mono<Void> cry(String reason, Runnable after);
}
