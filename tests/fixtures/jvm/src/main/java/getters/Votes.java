package getters;

import org.springframework.web.reactive.function.server.ServerRequest;

public class Votes {
    void upvote(ServerRequest request) {
        request.bodyToMono(Vote.class)
                .doOnNext(vote -> vote.group());
//                                     ^ d: src/main/java/getters/Votes.java:22
        request.bodyToFlux(Vote.class).map(vote -> vote.group());
//                                                      ^ d: src/main/java/getters/Votes.java:22
        request.bodyToMono(Ballot.class).map(vote -> vote.group());
//                                                        ^ d: !jump
        request.queryParam(Vote.class).map(vote -> vote.group());
//                                                      ^ d: !jump
    }

    interface Ballot {
    }

    public record Vote(
            String group,
            String plural) {}

    public record Poll(String group) {}
}
