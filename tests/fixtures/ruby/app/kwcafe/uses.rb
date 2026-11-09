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

  def ternary(flag)
    mugful = 1
    flag ? mugful : 2
    #      ^ d: app/kwcafe/uses.rb:25
  end

  def given
    mugful = 1
    puts mugful
    #    ^ d: app/kwcafe/uses.rb:31
  end
end
