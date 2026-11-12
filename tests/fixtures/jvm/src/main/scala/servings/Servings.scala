package servings

class Crock {
  def stew(): Unit = {
  }
}

class Wok {
  def stew(): Unit = {
  }
}

case class Kit(crock: Crock, wok: Wok)

object Servings {
  def servePair(): (Crock, Wok) = (new Crock, new Wok)

  def serveKit(): Kit = Kit(new Crock, new Wok)

  def unpack(): Unit = {
    val (c, w) = servePair()
    w.stew()
//    ^ d: src/main/scala/servings/Servings.scala:9
    c.stew()
//    ^ d: src/main/scala/servings/Servings.scala:4
    val Kit(kc, kw) = serveKit()
    kw.stew()
//     ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:9 (#1484)
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
}
