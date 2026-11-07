# Three Percolators: Moka's, Siphon's and the top-level one. `module Moka::Bar` does not see
# Moka's constants, `include Siphon` brings Siphon's in, and a class opened on the cursor's own
# line leaves the nesting unknown.
module Moka::Bar
  def self.go
    Percolator.new.drip
    #              ^ d: app/percolator.rb:2
    pot = Percolator.new
    pot.drip
    #   ^ d: app/percolator.rb:2
  end
end

class PercolatorMixer
  include Siphon

  def go
    Percolator.new.drip
    #              ^ d: app/percolator_siphon.rb:3
    pot = Percolator.new
    pot.drip
    #   ^ d: app/percolator_siphon.rb:3
  end
end

module Moka; class Barista; def go; Percolator.new.drip; end; end; end
#                                   ^ d: picker app/percolator_moka.rb:2, app/percolator_siphon.rb:2, app/percolator.rb:1
#                                                  ^ d: picker app/percolator_moka.rb:3, app/percolator_siphon.rb:3, app/percolator.rb:2
