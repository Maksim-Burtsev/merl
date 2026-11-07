package rebound

class Teapot:
  def boil(): Unit = ()
  def anon(): Unit = new Percolator { def z(): Unit = this.boil() }
  //                                                       ^ d: picker src/main/scala/rebound/Percolator.scala:4, …
  def after(): Unit = { new Percolator { def z(): Unit = () }; this.boil() }
  //                                                                ^ d: src/main/scala/rebound/Teapot.scala:4
  def lines(): Unit =
    new Percolator {
      def z(): Unit = this.boil()
      //                   ^ d: picker src/main/scala/rebound/Percolator.scala:4, …
    }
    this.boil()
    //   ^ d: src/main/scala/rebound/Teapot.scala:4
  def mixed(): Unit =
    new Percolator with Serializable {
      def z(): Unit = this.boil()
      //                   ^ d: picker src/main/scala/rebound/Percolator.scala:4, …
    }
