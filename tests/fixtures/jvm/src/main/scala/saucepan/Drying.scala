package saucepan

class Whisk

object Drying {
  implicit class WhiskDry(l: Whisk) {
    def dry(): Unit = ()
  }
  implicit class SaucepanDry(val s: Saucepan) extends AnyVal {
    def dry(): Unit = ()
  }
}
