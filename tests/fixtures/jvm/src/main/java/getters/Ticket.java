package getters;

import java.util.Optional;
import lombok.Data;
import lombok.EqualsAndHashCode;

@Data
@EqualsAndHashCode(callSuper = false)
@Deprecated(
        since = "2",
        forRemoval = false)
public class Ticket {
    private Spec spec;

    public static Ticket from(Account account) {
        return null;
    }

    public static Ticket of(Account account) {
        return null;
    }

    public static Account of(Ticket ticket) {
        return null;
    }

    public static Optional<Ticket> label(Account account) {
        return Optional.empty();
    }
}
