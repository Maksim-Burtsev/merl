module Moka
  class Percolator
    def drip
      1
    end

    def self.make
      Percolator.new.drip
      #              ^ d: app/percolator_moka.rb:3
    end
  end
end
