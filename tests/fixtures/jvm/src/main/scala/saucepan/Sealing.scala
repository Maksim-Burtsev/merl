package saucepan

class Ladle[A]

class Tongs

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
}
