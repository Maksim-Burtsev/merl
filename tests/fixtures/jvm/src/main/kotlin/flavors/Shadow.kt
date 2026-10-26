package flavors

class Outer(val mark: Int)

class Inner(val mark: Int)

class Catalog(val name: String) {
    companion object {
        val entries: List<Topic> = emptyList()
    }
}

fun looped(items: List<Outer>, others: List<Inner>) {
    items.forEach { w ->
        for (w in others) {
            println(w.mark)
//                    ^ d: picker src/main/kotlin/flavors/Shadow.kt:3, src/main/kotlin/flavors/Shadow.kt:5
        }
    }
}

fun caught(items: List<Outer>) {
    items.forEach { w ->
        try {
            println(w.mark)
        } catch (w: Inner) {
            println(w.mark)
//                    ^ d: src/main/kotlin/flavors/Shadow.kt:5
        }
    }
}

fun destructured(items: List<Outer>, pair: Pair<Int, Inner>) {
    items.forEach { w ->
        val (a, w) = pair
        if (a > 0) {
            println(w.mark)
//                    ^ d: picker src/main/kotlin/flavors/Shadow.kt:3, src/main/kotlin/flavors/Shadow.kt:5
        }
    }
}

fun typedInner(items: List<Outer>, others: List<Inner>) {
    items.forEach { w ->
        others.forEach { w: Inner ->
            println(w.mark)
//                    ^ d: picker src/main/kotlin/flavors/Shadow.kt:3, src/main/kotlin/flavors/Shadow.kt:5
        }
    }
}

fun crossesNoType(items: List<Outer>) {
    items.forEach { w ->
        class Local(val w: Inner) {
            fun show() {
                println(w.mark)
//                        ^ d: src/main/kotlin/flavors/Shadow.kt:5
            }
        }
    }
}

fun crossesNoFun(items: List<Outer>) {
    items.forEach { w ->
        fun show(w: Inner) {
            println(w.mark)
//                    ^ d: src/main/kotlin/flavors/Shadow.kt:5
        }
    }
}

fun notAnEnum() {
    Catalog.entries.forEach { topic ->
        println(topic.name)
//                    ^ d: picker src/main/kotlin/flavors/Shadow.kt:7, src/main/kotlin/flavors/Shelf.kt:3, src/main/kotlin/shop/warehouse/Warehouse.kt:8
    }
}
