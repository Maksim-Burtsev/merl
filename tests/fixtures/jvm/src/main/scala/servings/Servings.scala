package servings

class Crock {
  def stew(): Unit = {
  }
}

class Wok {
  def stew(): Unit = {
  }
}

case class Hamper(crock: Crock, wok: Wok)

object Servings {
  def servePair(): (Crock, Wok) = (new Crock, new Wok)

  def serveKit(): Hamper = Hamper(new Crock, new Wok)

  def unpack(): Unit = {
    val (c, w) = servePair()
    w.stew()
//    ^ d: src/main/scala/servings/Servings.scala:9
    c.stew()
//    ^ d: src/main/scala/servings/Servings.scala:4
    val Hamper(kc, kw) = serveKit()
    kw.stew()
//     ^ d: src/main/scala/servings/Servings.scala:9
    kc.stew()
//     ^ d: src/main/scala/servings/Servings.scala:4
    serveKit() match {
      case Hamper(g, h) =>
        h.stew()
//        ^ d: src/main/scala/servings/Servings.scala:9
        g.stew()
//        ^ d: src/main/scala/servings/Servings.scala:4
    }
    val (_, sw) = servePair()
    sw.stew()
//     ^ d: src/main/scala/servings/Servings.scala:9
  }

  def serveTriple(): (Crock, Wok, Crock) = (new Crock, new Wok, new Crock)

  def serveLoose() = (new Crock, new Wok)

  def unsure(w: Crock): Unit = {
    val (a, b, z) = servePair()
    z.stew()
//    ^ d: !jump
    val (l, m) = serveLoose()
    m.stew()
//    ^ d: !jump
    val (t1, t2, t3) = serveTriple()
    t2.stew()
//     ^ d: src/main/scala/servings/Servings.scala:9
    val (p, q) = serveTriple()
    q.stew()
//    ^ d: !jump
  }

  def shadowed(w: Crock): Unit = {
    val (c, w) = servePair()
    w.stew()
//    ^ d: src/main/scala/servings/Servings.scala:9
  }

  def serveBoxed(): (Crock, Option[Wok]) = (new Crock, Some(new Wok))

  def serveSplit():
    (Crock, Wok) = (new Crock, new Wok)

  def unread(): Unit = {
    val (bc, bw) = serveBoxed()
    bw.stew()
//     ^ d: !jump
    val (sc, sw) = serveSplit()
    sw.stew()
//     ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:9 (#1484)
    val (uc, uw) = Servings.servePair()
    uw.stew()
//     ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:9 (#1484)
  }

  def local(): Unit = {
    def servePair() = (new Wok, new Crock)
    val (c, w) = servePair()
    w.stew()
//    ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:4 (#1484)
  }
}

case class Shelf[T](held: T, wok: Wok)

case class Tote(crock: Crock = new Crock, wok: Wok)

case class Bag(crock: Crock, woks: Wok*)

object Unpacked {
  def kitted(kw: Crock, shelf: Shelf[Wok], tote: Tote, bag: Bag): Unit = {
    val Hamper(_, kw) = Hamper(new Crock, new Wok)
    kw.stew()
//     ^ d: src/main/scala/servings/Servings.scala:9
    val Shelf(sh, sw) = shelf
    sh.stew()
//     ^ d: !jump
    val Tote(tc, tw) = tote
    tc.stew()
//     ^ d: !jump
    tw.stew()
//     ^ d: src/main/scala/servings/Servings.scala:9
    val Bag(bc, bw) = bag
    bw.stew()
//     ^ d: !jump
    bc.stew()
//     ^ d: src/main/scala/servings/Servings.scala:4
  }
}
