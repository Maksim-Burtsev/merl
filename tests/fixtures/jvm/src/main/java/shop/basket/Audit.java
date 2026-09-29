package shop.basket;

import shop.pricing.Offer;
import shop.pricing.Receipt;

@interface Audited {}

@Audited
//^ d: src/main/java/shop/basket/Audit.java:6
public class Audit {
    static final String TEMPLATE = """
            public class Basket {
            """;

    private final Receipt receipt;
    //            ^ d: src/main/java/shop/pricing/Receipt.java:3

    public Audit(Receipt receipt) {
        this.receipt = receipt;
    }

    public static Audit of(Receipt r) {
        return new Audit(r);
        //         ^ d: picker src/main/java/shop/basket/Audit.java:10, src/main/java/shop/basket/Audit.java:18
    }

    Offer offer() {
    // ^ d: src/main/java/shop/pricing/Offer.java:3
        return Offer.PLAIN;
    }

    Basket basket() {
    // ^ d: picker src/main/java/shop/basket/Audit.java:12, src/main/java/shop/basket/Basket.java:15, src/main/java/shop/basket/Basket.java:25; want src/main/java/shop/basket/Basket.java:15 (#367)
        return null;
    }
}
