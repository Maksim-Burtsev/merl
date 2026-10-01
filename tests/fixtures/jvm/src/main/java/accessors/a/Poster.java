package accessors.a;

import lombok.AccessLevel;
import lombok.Data;
import lombok.Getter;

@Data
public class Poster {
    private String caption;
    private static int serial;
    private boolean pinned;
    @Getter(AccessLevel.NONE)
    private String secret;
}
