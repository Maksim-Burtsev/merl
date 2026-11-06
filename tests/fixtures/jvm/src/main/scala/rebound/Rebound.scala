package rebound

class Rebound(val x: Cup):
  def gen(ms: List[Mug]): Unit = for (x <- ms) x.sip()
  //                                             ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def gen3(ms: List[Mug]): Unit = for x <- ms do x.sip()
  //                                               ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def yields(ms: List[Mug]): List[Unit] = for { x <- ms } yield x.sip()
  //                                                              ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def genClosed(ms: List[Mug]): Unit = { for (x <- ms) x.sip(); x.sip() }
  //                                                     ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  //                                                              ^ d: src/main/scala/rebound/Cup.scala:4
  def genBlock(ms: List[Mug]): Unit =
    for (x <- ms) {
      x.sip()
      //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
    }
    x.sip()
    //^ d: src/main/scala/rebound/Cup.scala:4
  def genDo(ms: List[Mug]): Unit =
    for
      x <- ms
    do
      x.sip()
      //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
    x.sip()
    //^ d: src/main/scala/rebound/Cup.scala:4
  def genYield(ms: List[Mug]): List[Unit] =
    for {
      x <- ms
    } yield x.sip()
    //        ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def enumerated(ms: List[Mug]): List[Unit] =
    for
      m <- ms
      x = m
    yield x.sip()
    //      ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def arms(a: Any): Unit = a match { case x: Mug => x.sip(); case _ => x.sip() }
  //                                                  ^ d: src/main/scala/rebound/Mug.scala:4
  //                                                                     ^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def pair(): Unit =
    val (x, n) = (Mug(), 1)
    x.sip()
    //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def extract(b: Tray): Unit =
    val Tray(x) = b
    x.sip()
    //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def given1(): Unit =
    given x: Mug = Mug()
    x.sip()
    //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def implicit1(): Unit =
    implicit val x: Mug = Mug()
    x.sip()
    //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
  def colon(ms: List[Mug]): Unit =
    ms.foreach: x =>
      x.sip()
      //^ d: picker src/main/scala/rebound/Cup.scala:4, src/main/scala/rebound/Mug.scala:4
    x.sip()
    //^ d: src/main/scala/rebound/Cup.scala:4
  def comment(): Unit =
    // for (x <- ms)
    x.sip()
    //^ d: src/main/scala/rebound/Cup.scala:4
  def string(): Unit =
    val s = "x <- ms; val (x, y) = "; x.sip()
    //                                  ^ d: src/main/scala/rebound/Cup.scala:4
  def read(): Unit =
    val y = x; x.sip()
    //           ^ d: src/main/scala/rebound/Cup.scala:4
  def later(ms: List[Mug]): Unit =
    x.sip()
    //^ d: src/main/scala/rebound/Cup.scala:4
    for (x <- ms) ()
