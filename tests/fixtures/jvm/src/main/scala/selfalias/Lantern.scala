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
  }
}
