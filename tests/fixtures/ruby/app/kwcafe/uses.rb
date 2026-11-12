class KwServe
  def serve
    mugful = 5
    pot = Kwpot.new
    kwbrew(mugful: mugful)
    #      ^ d: picker app/kwcafe/brewer.rb:5, app/kwcafe/kwpot.rb:8
    #              ^ d: app/kwcafe/uses.rb:3
    pot.kwbrew(mugful: 4)
    #          ^ d: app/kwcafe/kwpot.rb:8
    pot.kwbrew mugful: 4
    #          ^ d: app/kwcafe/kwpot.rb:8
    kwbrew mugful: mugful
    #      ^ d: picker app/kwcafe/brewer.rb:5, app/kwcafe/kwpot.rb:8
    pot
  end
end

class KwNeighbours
  def kwshort mugful: 0
    #         ^ d: !jump
    mugful
  end

  def kwpair cups, mugful: 0, pot: nil
    mugful + cups
  end

  def kwlong cups,
             mugful: 0
    mugful + cups
  end

  def calls
    mugful = 3
    kwshort mugful: 1
    #       ^ d: app/kwcafe/uses.rb:19
    kwshort(mugful: mugful)
    #       ^ d: app/kwcafe/uses.rb:19
    kwpair 1, mugful: 2, pot: 3
    #         ^ d: app/kwcafe/uses.rb:24
    #                    ^ d: app/kwcafe/uses.rb:24
    kwpair(1, mugful: 2)
    #         ^ d: app/kwcafe/uses.rb:24
    self.kwpair 1, mugful: 2
    #              ^ d: app/kwcafe/uses.rb:24
    kwlong 1, mugful: 2
    #         ^ d: app/kwcafe/uses.rb:29
    kwpair 1, mugful: mugful
    #                 ^ d: app/kwcafe/uses.rb:34
    kwpair mugful ? 1 : 2, mugful: 3
    #                      ^ d: app/kwcafe/uses.rb:24
    puts kwshort 1, mugful: 2
    #               ^ d: app/kwcafe/uses.rb:19
    puts "a b", mugful: 2
    #           ^ d: none
  end

  def ternary(flag)
    mugful = 1
    flag ? mugful : 2
    #      ^ d: app/kwcafe/uses.rb:59
  end

  def given
    mugful = 1
    puts mugful
    #    ^ d: app/kwcafe/uses.rb:65
  end
end
