package shop.basket;

import shop.pricing.Coupon;
import shop.pricing.Offer;
import shop.pricing.Priced;
import shop.pricing.Pricing;
import shop.pricing.Receipt;
import shop.pricing.Tariff;
import shop.warehouse.Courier;

/*
 * A block comment that reads like code declares nothing:
 * public class Coupon {
 */
public class Basket implements Priced {
    //                         ^ d: src/main/java/shop/pricing/Priced.java:3
    static final int WEIGHT_LIMIT = 30;

    private final Tariff tariff;
    //            ^ d: src/main/java/shop/pricing/Tariff.java:4
    private final Coupon coupon = new Coupon();
    //                                ^ d: src/main/java/shop/pricing/Coupon.java:3
    public String owner = "guest";

    public Basket(Tariff tariff) {
        this.tariff = tariff;
        //   ^ d: src/main/java/shop/basket/Basket.java:19
    }

    public int gross() {
        return Pricing.discount(tariff.rate());
        //     ^ d: src/main/java/shop/pricing/Pricing.java:3
        //             ^ d: src/main/java/shop/pricing/Pricing.java:8
        //                      ^ d: src/main/java/shop/basket/Basket.java:19
        //                             ^ d: src/main/java/shop/pricing/Tariff.java:5
    }

    public int bonus() {
        return coupon.rate() + gross();
        //            ^ d: src/main/java/shop/pricing/Coupon.java:4
        //                     ^ d: src/main/java/shop/basket/Basket.java:30
    }

    @Override
    public int price() {
        return bonus();
    }

    public int restock(int gross) {
        return gross + WEIGHT_LIMIT;
        //     ^ d: src/main/java/shop/basket/Basket.java:49
        //             ^ d: src/main/java/shop/basket/Basket.java:17
    }

    public String describe(Tariff t, Coupon c) {
        return t.describe() + c.describe();
        //       ^ d: src/main/java/shop/pricing/Tariff.java:9
        //                      ^ d: src/main/java/shop/pricing/Coupon.java:8
    }

    public String ship() {
        Courier courier = new Courier("post");
        //                    ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:8
        return courier.label() + owner;
        //             ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:9
        //                       ^ d: src/main/java/shop/basket/Basket.java:23
    }

    public int receipt(Receipt r, Offer o) {
        return r.total() + (o == Offer.CUT ? 1 : 0);
        //       ^ d: src/main/java/shop/pricing/Receipt.java:5
        //                             ^ d: src/main/java/shop/pricing/Offer.java:5
    }

    public java.util.function.ToIntFunction<Tariff> rater() {
        return Tariff::rate;
        //             ^ d: src/main/java/shop/pricing/Tariff.java:5
        // status: rate → Tariff.rate (via import src/main/java/shop/pricing/Tariff.java)
    }
}
