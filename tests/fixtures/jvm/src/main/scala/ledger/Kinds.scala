package ledger

import scala.concurrent.ExecutionContext

object Folio:
  def tally(xs: List[Voucher]): Long = xs.map(_.amount).sum

case class Voucher(id: Long, amount: Long)

class Bill(val ticket: Long, var settled: Boolean) {
  override def toString = s"Bill($ticket)"
}

sealed abstract class Figure
case object Blank extends Figure
implicit class RichCount(x: Int)

trait Vault[F[_]]:
  private[ledger] def reload(): Unit
  inline def trace(msg: String): Unit = ()

package object teller {
  val quota = 10
  var clicks = 0
  lazy val engine = project
  implicit val ctx: ExecutionContext = ExecutionContext.global
}

type Ident = Long
opaque type PatronId = Long
trait Beast:
  type Pet <: Beast

case class Patron(nick: String, years: Int)

given patronOrder: Ordering[Patron] = Ordering.by(_.nick)
given patronShow: Show[Patron] with
  def show(p: Patron) = p.nick
given Ordering[Voucher] = Ordering.by(_.id)

extension (s: String) def slugify: String = s.toLowerCase

enum Hue:
  case Crimson, Azure, Olive

enum Orbit(mass: Double):
  case Venus extends Orbit(4.8e24)
  case Disc(r: Double)

case class Memo(
  remark: String,
)

object Wording:
  val sample = """
  def quota = 0
  case class Voucher(id: Long)
  """
  val `type` = 1
