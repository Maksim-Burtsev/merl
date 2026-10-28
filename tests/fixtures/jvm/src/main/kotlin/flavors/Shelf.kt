package flavors

data class Topic(val name: String)

data class Wrapped(val topic: Topic, val followed: Boolean)

class Plain(val topic: String)

fun shelf(items: List<Wrapped>) {
    items.forEach { wrapped ->
        val n = wrapped.topic.name
        item(key = n) {
            show(name = wrapped.topic.name)
//                              ^ d: src/main/kotlin/flavors/Shelf.kt:5
        }
    }
}
