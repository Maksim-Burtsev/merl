package brine

class Funnel[A] {
  def andThen(x: Int): Unit = ()
}

class Cheesecloth {
  def andThen(x: Int): Unit = ()
}

// f is a function from a Funnel, not a Funnel (#1499 review).
def decant(f: Funnel[Int] => Unit, g: Funnel[Int]): Unit =
  f.andThen(1)
  //^ d: !jump
  g.andThen(1)
  //^ d: src/main/scala/brine/Funnel.scala:4
