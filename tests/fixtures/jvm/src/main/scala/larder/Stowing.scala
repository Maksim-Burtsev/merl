package larder

object Stowing {
  def bale(): Bale = Bale(new Creel, new Coffer)

  def pair(): (Creel, Coffer) = (new Creel, new Coffer)

  def made(): Hod = new Coffer

  def unbaled(): Unit = {
    val Bale(c, d) = bale()
    d.stow()
//    ^ d: src/main/scala/larder/Coffer.scala:4
  }

  def matched(k: Bale): Unit = {
    k match {
      case Bale(g, h) => h.stow()
//                         ^ d: src/main/scala/larder/Coffer.scala:4
    }
  }

  def literal(): Unit = {
    val (e, f) = (new Creel, new Coffer)
    f.stow()
//    ^ d: src/main/scala/larder/Coffer.scala:4
    e.stow()
//    ^ d: src/main/scala/larder/Creel.scala:4
    val (p, q) = (new Creel, made())
    q.stow()
//    ^ d: !jump
  }

  def paired(): Unit = {
    pair() match {
      case (c, w) => w.stow()
//                     ^ d: src/main/scala/larder/Coffer.scala:4
    }
    pair() match {
      case (a, b, z) => z.stow()
//                        ^ d: !jump
    }
  }

  def opened(n: Option[Creel], o: Option[Hod], v: Hod): Unit = {
    n match {
      case Some(e) => e.stow()
//                      ^ d: src/main/scala/larder/Creel.scala:4
      case None => ()
    }
    o match {
      case Some(e) => e.stow()
//                      ^ d: src/main/scala/larder/Hod.scala:4
      case None => ()
    }
    v match {
      case Some(e) => e.stow()
//                      ^ d: !jump
      case _ => ()
    }
    n match {
      case Some(x) =>
        v match {
          case Some(e) => e.stow()
//                          ^ d: !jump
          case _ => ()
        }
      case None => ()
    }
    n match {
      case Some(x) => o match { case Some(e) => e.stow() }
//                                                ^ d: !jump
      case None => ()
    }
  }

  def inline(n: Option[Creel], k: Bale): Unit = {
    n match { case Some(e) => e.stow(); case None => () }
//                              ^ d: src/main/scala/larder/Creel.scala:4
    k match { case Bale(g, h) => h.stow() }
//                                 ^ d: src/main/scala/larder/Coffer.scala:4
    pair() match { case (c, w) => w.stow() }
//                                  ^ d: src/main/scala/larder/Coffer.scala:4
    k match { case Bale(g, h) => g.stow(); case Bale(h, g) => g.stow() }
//                                                              ^ d: !jump
    k match {
      case Bale(g, h) => n match { case Some(e) => h.stow() }
//                                                   ^ d: src/main/scala/larder/Coffer.scala:4
    }
  }

  def short(n: Option[Coffer]): Unit = n match { case Some(e) => e.stow() }
//                                                                 ^ d: src/main/scala/larder/Coffer.scala:4
}
