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
//                ^ d: src/main/scala/censer/Censer.scala:4
    Censer.shared.cense()
//                ^ d: src/main/scala/censer/Censer.scala:4
    Censer.make()
//         ^ d: src/main/scala/censer/Censer.scala:9
  }
}

object Rack {
  def run(): Unit = {
    Grill.make().cense()
//               ^ d: src/main/scala/censer/Censer.scala:13
  }
}
