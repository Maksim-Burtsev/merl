package selfalias

class Lantern { self =>
  var wick = 0
  def trim(): Unit = ()

  class Hood {
    var wick = 0
    def trim(): Unit = ()
    def run(): Unit = {
      self.wick += 1
//         ^ d: src/main/scala/selfalias/Lantern.scala:4
      self.trim()
//         ^ d: src/main/scala/selfalias/Lantern.scala:5
      this.wick += 1
//         ^ d: src/main/scala/selfalias/Lantern.scala:8
    }
    def light(self: Hood): Unit = {
      self.wick += 1
//         ^ d: src/main/scala/selfalias/Lantern.scala:8
    }
    def dim(): Unit = {
      val self = new Hood
      self.wick += 1
//         ^ d: src/main/scala/selfalias/Lantern.scala:8
    }
    def flick(): Unit = List(new Hood).foreach { self =>
      self.wick += 1
//         ^ d: src/main/scala/selfalias/Lantern.scala:8
    }
    def snap(self: Hood) = self.wick
//                              ^ d: src/main/scala/selfalias/Lantern.scala:8
  }
}
