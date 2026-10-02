// Every construct `f` folds in Kotlin, and the cases that once broke it.
package a

import a.B  // f: 4-6
// a comment inside the imports  // f: 4-6
import a.C  // f: 4-6

class A(  // f: 8-9
    val x: Int,  // f: 8-9
) : B(), C {  // f: 10-84
    init {  // f: 11-13
        go()  // f: 10-84
    }  // f: 10-84

    constructor(y: String) : this(  // f: 15-19
        1,  // f: 15-19
    ) {  // f: 15-19
        go()  // f: 15-19
    }  // f: 15-19

    val p: Int  // f: 10-84
        get() {  // f: 22-24
            return 1  // f: 22-24
        }  // f: 22-24

    fun f(x: Int): Int {  // f: 26-71
        if (x > 0) {  // f: 27-28
            return 1  // f: 26-71
        } else {  // f: 29-31
            return 2  // f: 26-71
        }  // f: 26-71
        try {  // f: 26-71
            go()  // f: 26-71
        } catch (e: Exception) {  // f: 26-71
            go()  // f: 26-71
        } finally {  // f: 26-71
            go()  // f: 26-71
        }  // f: 26-71
        val r = when (x) {  // f: 39-44
            1 -> {  // f: 40-42
                2  // f: 26-71
            }  // f: 26-71
            else -> 3  // f: 26-71
        }  // f: 26-71
        list.map { y ->  // f: 45-47
            y + 1  // f: 26-71
        }  // f: 26-71
        foo(  // f: 26-71
            1,  // f: 26-71
        ) {  // f: 50-52
            it  // f: 26-71
        }  // f: 26-71
        val s = "a ${ mapOf(1 to "}") } b"  // f: 26-71
        val t = """  // f: 26-71
            { not a block  // f: 26-71
        """  // f: 26-71
        val u = when (x) {  // f: 57-65
            is Int ->  // f: 26-71
                listOf(  // f: 59-61
                    1,  // f: 26-71
                )  // f: 26-71

            in 0..2 -> 4  // f: 26-71
            else -> 5  // f: 26-71
        }  // f: 26-71
        for (i in 0..2)  // f: 26-71
            println(  // f: 26-71
                i  // f: 26-71
            )  // f: 26-71
        return r  // f: 26-71
    }  // f: 26-71

    fun g() =  // f: 73-76
        listOf(  // f: 73-76
            1,  // f: 73-76
        )  // f: 73-76

    companion object {  // f: 78-83
        val z = object : Runnable {  // f: 79-82
            override fun run() {  // f: 80-81
            }  // f: 80-81
        }  // f: 79-82
    }  // f: 78-83
}  // f: 10-84

data class Box<T>(  // f: 86-87
    val value: T,  // f: 86-87
) {  // f: 88-92
    constructor(other: Box<T>) : this(  // f: 89-91
        other.value,  // f: 89-91
    )  // f: 89-91
}  // f: 88-92

fun <T> Box<T>.pair(): Map<T, List<T>> =  // f: 94-97
    mapOf(  // f: 94-97
        value to listOf(value),  // f: 94-97
    )  // f: 94-97

enum class E {  // f: 99-102
    X,  // f: 99-102
    Y,  // f: 99-102
}  // f: 99-102
