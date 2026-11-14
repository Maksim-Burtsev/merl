package servings

case class Rig(crock: Crock, wok: Wok)

object Rig {
  def unapply(r: Rig): Option[(Wok, Crock)] = Some((r.wok, r.crock))
}

object Rigs {
  def rigged(rig: Rig): Unit = {
    val Rig(rw, rc) = rig
    rw.stew()
//     ^ d: !jump
  }
}
