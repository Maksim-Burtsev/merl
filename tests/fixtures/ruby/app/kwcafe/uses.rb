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

class KwContinued < KwNeighbours
  def continued
    mugful = 3
    kwpair 1,
      mugful: 2
    # ^ d: app/kwcafe/uses.rb:24
    kwpair 1, pot: 2,
              mugful: mugful
    #         ^ d: app/kwcafe/uses.rb:24
    #                 ^ d: app/kwcafe/uses.rb:73
    x = kwpair 1,
               mugful: 2
    #          ^ d: app/kwcafe/uses.rb:24
    kwpair "a b", mugful: 2
    #             ^ d: app/kwcafe/uses.rb:24
    kwpair 'a', mugful: 2
    #           ^ d: app/kwcafe/uses.rb:24
    kwpair "#{mugful}", mugful: 2
    #                   ^ d: app/kwcafe/uses.rb:24
    x
  end
end

class KwBlocks < KwNeighbours
  def blocks(items)
    mugful = 3
    items.each { |i| kwpair 1,
      mugful: 2 }
    # ^ d: app/kwcafe/uses.rb:24
    items.each { |i| kwpair i, mugful: 2 }
    #                          ^ d: app/kwcafe/uses.rb:24
    items.each do |i|
      kwpair i,
        mugful: 2
      # ^ d: app/kwcafe/uses.rb:24
    end
    items.map { |i| kwpair i,
                      mugful: mugful }
    #                 ^ d: app/kwcafe/uses.rb:24
    #                         ^ d: app/kwcafe/uses.rb:96
    items.each { |i| kwpair(i,
      mugful: 2) }
    # ^ d: app/kwcafe/uses.rb:24
    items.each { |i| puts i,
      mugful: 2 }
    # ^ d: none
    { mugful: 2,
      pot: mugful }
    #      ^ d: app/kwcafe/uses.rb:96
    mugful
  end
end

class KwBlockNeighbours < KwNeighbours
  def neighbours(items)
    mugful = 3
    3.times { kwpair 1, mugful: 2 }
    #                   ^ d: app/kwcafe/uses.rb:24
    items.each { |i| kwshort mugful: i }
    #                        ^ d: app/kwcafe/uses.rb:19
    items.map { |i| { mugful: i, pot: 2 } }
    #                 ^ d: none
    #                            ^ d: none
    items.each { |i, mugful: 1| kwpair i, mugful: mugful }
    #                ^ d: app/kwcafe/uses.rb:126; want app/kwcafe/uses.rb:134 (#1446)
    #                                     ^ d: app/kwcafe/uses.rb:24
    items.each { |i| kwpair i, mugful }
    #                          ^ d: app/kwcafe/uses.rb:126
    mugful
  end
end
