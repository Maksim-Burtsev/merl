package shop.basket

import shop.pricing.Tariff
import shop.warehouse.Courier
import shop.warehouse.Depot
import shop.warehouse.weigh

typealias Grams = Int

const val LIMIT = 30

enum class Speed { SLOW, FAST }

interface Tracker {
    fun track(id: String): Boolean
}

sealed class Outcome {
    data class Delivered(val grams: Grams) : Outcome()
    object Lost : Outcome()
}

fun checkout(basket: Basket): Int {
    val courier = Courier("post")
    //            ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:8
    return basket.gross() + weigh(courier.name.length) + Depot.open()
    //            ^ d: src/main/java/shop/basket/Basket.java:30
    //                      ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:12
    //                                    ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:8
    //                                                     ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:14
    //                                                           ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:15
}

fun Tariff.doubled(): Int = rate() * 2

fun useExtension(t: Tariff): Int = t.doubled()
//                                   ^ d: picker src/main/kotlin/shop/basket/Checkout.kt:34, src/main/kotlin/shop/warehouse/Warehouse.kt:20; want src/main/kotlin/shop/basket/Checkout.kt:34 (#362)

private fun limit(): Grams = LIMIT
//                   ^ d: src/main/kotlin/shop/basket/Checkout.kt:8
//                           ^ d: src/main/kotlin/shop/basket/Checkout.kt:10

fun track(tracker: Tracker, speed: Speed): Boolean = tracker.track(speed.name) && limit() > 0
//                 ^ d: src/main/kotlin/shop/basket/Checkout.kt:14
//                                 ^ d: src/main/kotlin/shop/basket/Checkout.kt:12
//                                                           ^ d: src/main/kotlin/shop/basket/Checkout.kt:15
//                                                                                ^ d: picker src/main/kotlin/shop/basket/Checkout.kt:39
// status: limit: by name, 1 match

fun outcome(o: Outcome): Int = when (o) {
    is Outcome.Delivered -> o.grams
    //         ^ d: src/main/kotlin/shop/basket/Checkout.kt:19
    Outcome.Lost -> 0
    //      ^ d: src/main/kotlin/shop/basket/Checkout.kt:20
}

suspend fun deliver(grams: Grams): Int = weigh(grams)
//                                       ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:12

fun fast(speed: Speed): Boolean = speed == Speed.FAST
//                                               ^ d: src/main/kotlin/shop/basket/Checkout.kt:12

enum class Lane(val days: Int) {
    ROAD(3),
    // A comment between the constants.
    AIR(1) {
        override fun toString() = "air"
    };

    fun late() = days > 2
}

fun air(): Lane = Lane.AIR
//                     ^ d: src/main/kotlin/shop/basket/Checkout.kt:66

fun hire(): String = Courier(name = "post").name
//                           ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:8

fun heavy(): Int = weigh(
    grams = 3,
//  ^ d: src/main/kotlin/shop/warehouse/Warehouse.kt:12
)

fun loud(): Unit = println(message = "x")
//                         ^ d: none
//                         status: message: argument label
