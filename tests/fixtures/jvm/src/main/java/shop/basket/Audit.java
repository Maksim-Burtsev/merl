package shop.basket;

import shop.pricing.Offer;
import shop.pricing.Receipt;
import java.util.concurrent.TimeUnit;

@interface Audited {}

@Audited
//^ d: src/main/java/shop/basket/Audit.java:7
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
        //         ^ d: picker src/main/java/shop/basket/Audit.java:11, src/main/java/shop/basket/Audit.java:19
    }

    Offer offer() {
    // ^ d: src/main/java/shop/pricing/Offer.java:3
        return Offer.PLAIN;
        //           ^ d: src/main/java/shop/pricing/Offer.java:4
    }

    Offer half() {
        return Offer.HALF;
        //           ^ d: src/main/java/shop/pricing/Offer.java:8
    }

    long seconds(long ms) {
        return TimeUnit.MILLISECONDS.toSeconds(ms);
        //              ^ d: none
    }

    Basket basket() {
    // ^ d: picker src/main/java/shop/basket/Basket.java:15, src/main/java/shop/basket/Basket.java:25; want src/main/java/shop/basket/Basket.java:15 (#367)
        return null;
    }
}
