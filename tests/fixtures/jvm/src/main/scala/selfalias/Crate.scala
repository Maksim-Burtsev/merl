package selfalias

class Crate { outer =>
  def pry(): Unit = ()

  class Lid {
    def pry(): Unit = ()
    val hinge: Lid = this
    def run(): Unit = {
      outer.pry()
//          ^ d: src/main/scala/selfalias/Crate.scala:4
      Crate.this.pry()
//               ^ d: src/main/scala/selfalias/Crate.scala:4
      Lid.this.pry()
//             ^ d: src/main/scala/selfalias/Crate.scala:7
    }
    def pass(outer: Lid): Unit = outer.pry()
//                                     ^ d: src/main/scala/selfalias/Crate.scala:7
    def keep(): Unit = {
      val outer = new Lid
      outer.pry()
//          ^ d: src/main/scala/selfalias/Crate.scala:7
    }
    def each(): Unit = List(new Lid).foreach { outer =>
      outer.pry()
//          ^ d: picker src/main/scala/selfalias/Crate.scala:4, src/main/scala/selfalias/Crate.scala:7, src/main/scala/selfalias/Crate.scala:38
    }
    def pick(x: Any): Unit = x match {
      case outer: Lid => outer.pry()
//                             ^ d: src/main/scala/selfalias/Crate.scala:7
      case _ => ()
    }
    def loop(): Unit = for (outer <- List(new Lid)) outer.pry()
//                                                        ^ d: picker src/main/scala/selfalias/Crate.scala:4, src/main/scala/selfalias/Crate.scala:7, src/main/scala/selfalias/Crate.scala:38
  }

  class Hinge { lid =>
    def pry(): Unit = ()
    def run(): Unit = {
      outer.pry()
//          ^ d: src/main/scala/selfalias/Crate.scala:4
      lid.pry()
//        ^ d: src/main/scala/selfalias/Crate.scala:38
    }
  }

  val anon = new Runnable {
    def pry(): Unit = ()
    def run(): Unit = {
      outer.pry()
//          ^ d: src/main/scala/selfalias/Crate.scala:4
    }
  }
}
