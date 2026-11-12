package censer

class Censer {
  def cense(): Unit = ()
}

object Censer {
  val shared = new Censer
  def make(): Censer = new Censer
}

class Grill {
  def cense(): Unit = ()
}

object Grill {
  def make(): Grill = new Grill
}

object Swing {
  def run(): Unit = {
    Censer.make().cense()
//                ^ d: picker src/main/scala/censer/Censer.scala:4, src/main/scala/censer/Censer.scala:13; want src/main/scala/censer/Censer.scala:4 (#1489)
    Censer.shared.cense()
//                ^ d: picker src/main/scala/censer/Censer.scala:4, src/main/scala/censer/Censer.scala:13; want src/main/scala/censer/Censer.scala:4 (#1489)
    Censer.make()
//         ^ d: src/main/scala/censer/Censer.scala:9
  }
}
