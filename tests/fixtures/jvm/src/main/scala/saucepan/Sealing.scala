package saucepan

class Ladle[A] { def stir(): Unit = () }

class Tongs { def stir(): Unit = () }

object Sealing {
  implicit class LadleSeal[A](l: Ladle[A]) {
    def clasp(): Unit = ()
  }
  implicit class TongsSeal(t: Tongs) {
    def clasp(): Unit = ()
  }
  implicit class AnyRinse[+A](a: A) {
    def swab(): Unit = ()
  }
  implicit class TongsRinse(t: Tongs) {
    def swab(): Unit = ()
  }
  implicit class LadleRub(l: Ladle[Int]) {
    def rub(): Unit = ()
  }
  implicit class TongsRub(t: Tongs) {
    def rub(): Unit = ()
  }
  implicit class LadleWipe[A <: Tongs](l: Ladle[A]) {
    def wipe(): Unit = ()
  }
  implicit class TongsWipe(t: Tongs) {
    def wipe(): Unit = ()
  }
}
