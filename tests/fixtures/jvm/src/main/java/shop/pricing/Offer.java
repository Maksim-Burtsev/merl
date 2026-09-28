package shop.pricing;

public enum Offer {
    PLAIN,
    CUT,
    /* A constant with a body, under an annotation. */
    @Deprecated
    HALF {
        int share() { return 50; }
    };

    int share() { return 0; }
}
