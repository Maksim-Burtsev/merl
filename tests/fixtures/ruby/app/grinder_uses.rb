# A class or module opened and closed again before the cursor on its line leaves the nesting
# known, and one still open there hides it only from a name the project declares more than once.
class PercolatorStand; end; Percolator.new.drip
#                           ^ d: app/percolator.rb:1
#                                          ^ d: app/percolator.rb:2

module Moka
  class Filter; end; Percolator.new.drip
  #                  ^ d: app/percolator_moka.rb:2
  #                                 ^ d: app/percolator_moka.rb:3
end

class GrinderStand; def go = Grinder.new.grind; end
#                                        ^ d: app/grinder.rb:2
class GrinderRack; def go; Grinder.new.grind; end; end
#                                      ^ d: app/grinder.rb:2
module GrinderShelf; def self.go = Moka::Mill.new.grind; end
#                                                 ^ d: app/grinder.rb:9

# Ruby does not see Moka's constants inside `class Moka::Hopper`; `d` keeps master's answer.
class Moka::Hopper
  def go
    Mill.new.grind
    # ^ d: app/grinder.rb:8
    #        ^ d: app/grinder.rb:9
  end
end
