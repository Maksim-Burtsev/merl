package getters;

import lombok.Data;

@Data
public class Account {
    private Spec spec;

    public Meta getMetadata() {
        return null;
    }
}
