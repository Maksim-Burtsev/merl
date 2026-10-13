@file:JvmName("Ledgers")

package com.example.ledger

import kotlin.math.abs
import java.time.Instant as Moment
import com.example.util.*

/**
 * A ledger keeps entries in order.
 * @param owner who keeps it
 */
@Suppress("unused")
data class Entry(val id: Long, val amount: Double, val note: String? = null)

enum class Kind(val sign: Int) { CREDIT(1), DEBIT(-1) }

sealed interface Event {
    object Opened : Event
    data class Moved(val entry: Entry) : Event
}

typealias Handler = (Event) -> Unit

const val LIMIT = 1_000
private val PATTERN = "[a-z]+\\d".toRegex()
internal val strict = Regex("^\\w+$")

abstract class Base<T : Comparable<T>>(protected open val owner: String) {
    abstract fun total(): T
    open fun describe(): String = "ledger of $owner"
}

class Ledger(override val owner: String) : Base<Double>(owner), Iterable<Entry> {
    private val entries = mutableListOf<Entry>()
    var handler: Handler? = null
        private set
    lateinit var opened: Moment

    companion object {
        @JvmStatic
        fun empty(): Ledger = Ledger("nobody")
    }

    init {
        require(owner.isNotBlank()) { "owner is blank" }
    }

    override fun iterator(): Iterator<Entry> = entries.iterator()

    override fun total(): Double = entries.sumOf { it.amount }

    fun add(amount: Double, note: String? = null): Entry {
        val entry = Entry(entries.size.toLong(), amount, note)
        entries += entry
        handler?.invoke(Event.Moved(entry))
        return entry
    }

    suspend fun sync(vararg ids: Long) {
        for (id in ids) {
            if (id !in 0..LIMIT) continue
            println("synced ${id} of ${entries.size}")
        }
    }

    fun kind(entry: Entry): Kind = when {
        entry.amount < 0 -> Kind.DEBIT
        entry.amount == 0.0 -> throw IllegalStateException("zero")
        else -> Kind.CREDIT
    }

    inline fun <reified R> find(crossinline test: (Entry) -> Boolean): R? =
        entries.firstOrNull { test(it) } as? R

    fun report(): String {
        val lines = entries.map { e ->
            "%-10s %8.2f".format(e.note ?: "-", abs(e.amount))
        }
        val raw = """
            |total: ${total()}
            |count: ${entries.size}
        """.trimMargin()
        return lines.joinToString("\n") + raw
    }

    fun check(x: Any): Boolean {
        val ch = 'a'
        val big = 0xFF_FFL
        val ratio = 1.5e3f
        return x is String && x.length > 0 || ch == '\n' || big > 0 || ratio != 0f
    }

    override fun toString() = "Ledger(owner=$owner, size=${entries.size})"
}

fun main(args: Array<String>) {
    val ledger = Ledger.empty().apply { add(10.0, "start") }
    try {
        ledger.add(-2.5)
    } catch (e: Exception) {
        return
    } finally {
        println(ledger)
    }
    // done
}
