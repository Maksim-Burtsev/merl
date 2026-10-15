package ledger.money

import ledger.core.{Entry, Ledger}

enum Currency(val code: String):
  case Euro extends Currency("EUR")
  case Dollar extends Currency("USD")
  case Other(name: String) extends Currency(name)

opaque type Cents = Long

object Cents:
  def apply(n: Long): Cents = n
  extension (c: Cents)
    def +(other: Cents): Cents = c + other
    inline def toLong: Long = c

trait Show[A]:
  def show(a: A): String

given Show[Entry] with
  def show(e: Entry): String = s"${e.name}: ${e.amount}"

def render[A](a: A)(using s: Show[A]): String = s.show(a)

infix def plus(a: Int, b: Int): Int = a + b

transparent inline def pick(flag: Boolean) = if flag then 1 else "one"

open class Account(val owner: String) derives CanEqual:
  def balance(l: Ledger): Long =
    l.total match
      case n if n > 0 => n
      case _          => 0L
  end balance
end Account

@main def run(): Unit =
  val l = Ledger(Entry("rent", -900), Entry("salary", 3000))
  for e <- List(1, 2, 3) do println(e)
  val sum = List(1, 2, 3).map(_ * 2).filter(x => x > 2).sum
  println(render(Entry("x", 1)))
  println(this.toString)
