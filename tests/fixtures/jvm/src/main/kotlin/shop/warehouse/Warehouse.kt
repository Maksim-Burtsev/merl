package shop.warehouse

val BANNER = """
class Courier(
fun weigh(grams: Int): Int {
"""

class Courier(val name: String) {
    fun label(): String = name
}

fun weigh(grams: Int): Int = grams / 1000

object Depot {
    fun open(): Int = 1
}

private fun limit(): Int = 1

fun String.doubled(): Int = length * 2
