package ledger.core

import scala.collection.mutable
import scala.concurrent.{ExecutionContext, Future}
import java.time._

/** One line of the ledger. */
final case class Entry(name: String, amount: Long, tags: List[String] = Nil)

sealed trait Kind
object Kind {
  case object Credit extends Kind
  case object Debit extends Kind
}

abstract class Store[A](val label: String) { self =>
  protected def all: Seq[A]
  def size: Int = all.size
}

class Ledger(entries: mutable.Buffer[Entry]) extends Store[Entry]("ledger") with Serializable {
  override protected def all: Seq[Entry] = entries.toSeq
  private lazy val index: Map[String, Entry] = entries.map(e => e.name -> e).toMap
  var limit: Int = 1024
  val ratio = 1.5e3
  val mask = 0xff
  val letter = 'z'

  def total: Long = entries.foldLeft(0L)(_ + _.amount)

  def find(name: String): Option[Entry] = index.get(name)

  def kind(e: Entry): Kind = if (e.amount > 0) Kind.Credit else Kind.Debit

  def describe(k: Kind): String = k match {
    case Kind.Credit => "credit"
    case Kind.Debit  => "debit"
  }

  @deprecated("use report", "0.2")
  def dump(): Unit = println(s"${entries.size} entries, total $total")

  def report(implicit ec: ExecutionContext): Future[String] = Future {
    val lines = for {
      e <- entries
      if e.amount != 0
    } yield f"${e.name}%-10s ${e.amount}%8d"
    lines.mkString("\n")
  }

  def safe(n: Long): Long =
    try {
      if (n < 0) throw new IllegalArgumentException("negative")
      n * 2
    } catch {
      case _: ArithmeticException => 0L
    } finally {
      limit -= 1
    }

  def loop(): Unit = {
    var i = 0
    while (i < limit) {
      i += 1
      if (i % 2 == 0) return
    }
  }
}

object Ledger {
  type Name = String
  def apply(xs: Entry*): Ledger = new Ledger(mutable.Buffer(xs: _*))
  val empty: Ledger = Ledger()
  val nothing: String = null
  val raw = """a "quoted"
    |line""".stripMargin
}
