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
//    ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:9 (#1484)
    c.stew()
//    ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:4 (#1484)
    val Kit(kc, kw) = serveKit()
    kw.stew()
//     ^ d: picker src/main/scala/servings/Servings.scala:4, src/main/scala/servings/Servings.scala:9, …; want src/main/scala/servings/Servings.scala:9 (#1484)
  }
}
