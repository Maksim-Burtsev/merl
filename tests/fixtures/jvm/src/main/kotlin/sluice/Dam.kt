package sluice.dam

open class Basin {
    open fun drain() {}
}

open class Sump : Basin() {
    override fun drain() {}
}

open class Tank : Sump()

open class Pond : Basin() { override fun drain() {} }

class Dam : Basin() {
    override fun drain() {}
    inner class Inner : Sump() {
        fun go() {
            drain()
        //  ^ d: src/main/kotlin/sluice/Dam.kt:8
        }
    }
    inner class Deeper : Tank() {
        fun go() {
            drain()
        //  ^ d: src/main/kotlin/sluice/Dam.kt:8
        }
    }
    inner class Plain {
        fun go() {
            drain()
        //  ^ d: src/main/kotlin/sluice/Dam.kt:16
        }
    }
    fun make(): Any = object : Sump() {
        fun go() {
            drain()
        //  ^ d: src/main/kotlin/sluice/Dam.kt:8
        }
    }
    fun use() {
        Pond().drain()
    //         ^ d: src/main/kotlin/sluice/Dam.kt:13
    }
}

open class Gauge {
    var sill = 0
}

class Reset(val clear: () -> Unit = { sill = 0 }) : Gauge() {
    fun run() {
        println(sill)
    //          ^ d: src/main/kotlin/sluice/Dam.kt:48
    }
}
