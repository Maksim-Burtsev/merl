package getters;

import lombok.Getter;

@SuppressWarnings(
        value = "unused")
public class Draft {
    private Spec spec;

    @Getter
    private Owner owner = new Owner(
    );
    private Spec copy;
}
