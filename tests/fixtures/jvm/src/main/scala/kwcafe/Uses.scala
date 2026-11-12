package kwcafe

import kwcafe.Brewer.kwbrew

object Uses {
  def serve(): Int = {
    val mugful = 5
    val pot = new Kwpot()
    kwbrew(mugful = mugful)
    //     ^ d: src/main/scala/kwcafe/Brewer.scala:6
    //              ^ d: src/main/scala/kwcafe/Uses.scala:7
    pot.kwbrew(mugful = 4)
    //         ^ d: picker src/main/scala/kwcafe/Pot.scala:6, …
    kwbrew(mugful)
    //     ^ d: src/main/scala/kwcafe/Uses.scala:7
    0
  }
}
