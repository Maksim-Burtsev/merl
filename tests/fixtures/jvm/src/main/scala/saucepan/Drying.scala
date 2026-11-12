package saucepan

class Ladle

object Drying {
  implicit class LadleDry(l: Ladle) {
    def dry(): Unit = ()
  }
  implicit class SaucepanDry(val s: Saucepan) extends AnyVal {
    def dry(): Unit = ()
  }
}
