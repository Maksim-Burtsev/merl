package app

class Basket {
    private val items = mutableListOf<String>()

    fun add(item: String) {
        items.add(item)
    }

    fun contains(item: String): Boolean = items.contains(item)
}
