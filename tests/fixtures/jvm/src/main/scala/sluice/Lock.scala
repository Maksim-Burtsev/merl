package sluice.lock

class Reach {
  def drain(): Unit = ()
}

class Pound extends Reach {
  override def drain(): Unit = ()
}

class Flume extends Reach { override def drain(): Unit = () }

class Lock extends Reach {
  override def drain(): Unit = ()
  class Inner extends Pound {
    def go(): Unit = {
      drain()
    //^ d: src/main/scala/sluice/Lock.scala:8
    }
  }
  def make(): Any = new Pound {
    def go(): Unit = {
      drain()
    //^ d: src/main/scala/sluice/Lock.scala:8
    }
  }
  def use(): Unit = {
    new Flume().drain()
  //            ^ d: src/main/scala/sluice/Lock.scala:11
  }
}
