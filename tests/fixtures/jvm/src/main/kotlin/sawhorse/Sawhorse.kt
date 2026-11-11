package sawhorse

open class Sawhorse<T>(val held: T) {
    fun hoist(): T = held
    fun <V> grip(v: V): V = v
}
