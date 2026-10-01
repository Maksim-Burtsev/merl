package ledger

import ledger.{Voucher, Folio => F}
//              ^ d: src/main/scala/ledger/Kinds.scala:8

class Uses(pair: (Int, Int)) extends Figure with Vault[List]:
//                                   ^ d: src/main/scala/ledger/Kinds.scala:14
//                                               ^ d: src/main/scala/ledger/Kinds.scala:18
  export Folio.tally
  //     ^ d: src/main/scala/ledger/Kinds.scala:5
  //           ^ d: src/main/scala/ledger/Kinds.scala:6
  val made = new Voucher(1, 2)
  //             ^ d: src/main/scala/ledger/Kinds.scala:8
  val bill = Bill(3, false)
  //         ^ d: src/main/scala/ledger/Kinds.scala:10
  def paid(v: Voucher, b: Bill): Boolean = b.settled && b.ticket > v.amount
  //          ^ d: src/main/scala/ledger/Kinds.scala:8
  //                                         ^ d: src/main/scala/ledger/Kinds.scala:10
  //                                                       ^ d: src/main/scala/ledger/Kinds.scala:10
  //                                                                  ^ d: src/main/scala/ledger/Kinds.scala:8
  def sum(xs: List[Voucher]): Long = Folio.tally(xs)
  //                                       ^ d: src/main/scala/ledger/Kinds.scala:6
  def shown: String = Blank.toString + RichCount(1).toString
  //                  ^ d: src/main/scala/ledger/Kinds.scala:15
  //                                   ^ d: src/main/scala/ledger/Kinds.scala:16
  def kept(v: Vault[List]): Unit = { v.reload(); v.trace("x") }
  //                                   ^ d: src/main/scala/ledger/Kinds.scala:19
  //                                               ^ d: src/main/scala/ledger/Kinds.scala:20
  def counts: Int = quota + clicks + engine.hashCode + ctx.hashCode
  //                ^ d: src/main/scala/ledger/Kinds.scala:23
  //                        ^ d: src/main/scala/ledger/Kinds.scala:24
  //                                 ^ d: src/main/scala/ledger/Kinds.scala:25
  //                                                   ^ d: src/main/scala/ledger/Kinds.scala:26
  def ids(i: Ident, p: PatronId, x: Beast#Pet): Unit = ()
  //         ^ d: src/main/scala/ledger/Kinds.scala:29
  //                   ^ d: src/main/scala/ledger/Kinds.scala:30
  //                                      ^ d: src/main/scala/ledger/Kinds.scala:32
  def who(p: Patron): String = p.nick + p.years
  //         ^ d: src/main/scala/ledger/Kinds.scala:34
  //                             ^ d: src/main/scala/ledger/Kinds.scala:34
  //                                      ^ d: src/main/scala/ledger/Kinds.scala:34
  def order = (patronOrder, patronShow, "Ab".slugify)
  //           ^ d: src/main/scala/ledger/Kinds.scala:36
  //                        ^ d: src/main/scala/ledger/Kinds.scala:37
  //                                         ^ d: src/main/scala/ledger/Kinds.scala:41
  def hue(h: Hue): Int = h match
    case Hue.Crimson | Hue.Azure => 1
    //       ^ d: src/main/scala/ledger/Kinds.scala:44
    //                     ^ d: src/main/scala/ledger/Kinds.scala:44
    case Olive => 2
    //   ^ d: src/main/scala/ledger/Kinds.scala:44
  def orbit(o: Orbit): Double = o match
    case Orbit.Venus => 1.0
    //         ^ d: src/main/scala/ledger/Kinds.scala:47
    case Disc(r) => r
    //   ^ d: src/main/scala/ledger/Kinds.scala:48
  def matched(v: Voucher): Long = v match
    case Voucher(id, _) => id
    //   ^ d: src/main/scala/ledger/Kinds.scala:8
  def remarks(m: Memo): String = remark
  //                             ^ d: none
  def typed: Int = Wording.`type`
  //                        ^ d: src/main/scala/ledger/Kinds.scala:59
  def split: Int =
    val (left, right) = pair
    left + right
  def stated(st: Statement, xs: List[Voucher]): Long = st.sum(xs)
  //             ^ d: src/main/java/ledger/Statement.java:3
  //                                                      ^ d: src/main/java/ledger/Statement.java:9
