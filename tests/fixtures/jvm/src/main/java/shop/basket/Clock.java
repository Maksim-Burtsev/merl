package shop.basket;

import java.util.concurrent.*;
import shop.pricing.*;

// #457: a type of a package imported whole is the one the file sees.
class Clock {
    long secs(long ms) {
        return TimeUnit.SECONDS.toMillis(ms);
        //              ^ d: none
    }

    Offer cut() {
        return Offer.CUT;
        //           ^ d: src/main/java/shop/pricing/Offer.java:5
    }
}
