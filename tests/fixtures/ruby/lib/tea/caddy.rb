# `self.class.m` in an instance method is the class's own `def self.m`, and `Const.instance.m`
# the instance method of a class that includes `Singleton` (#1097).
module Tea
  class Caddy
    include Singleton

    def self.blend(leaves)
      leaves
    end

    def scoop(leaves)
      leaves
    end

    def fill
      self.class.blend(1)
      #          ^ d: lib/tea/caddy.rb:7
    end

    def self.refill
      self.class.blend(1)
      #          ^ d: !jump
    end
  end

  class Tin
    def self.blend(leaves)
      leaves
    end

    def scoop(leaves)
      leaves
    end
  end

  module Measuring
    def measure
      self.class.blend(1)
      #          ^ d: !jump
    end
  end

  class Pot
    def brew
      Caddy.instance.scoop(1)
      #              ^ d: lib/tea/caddy.rb:11
      Tin.instance.scoop(1)
      #            ^ d: !jump
    end
  end
end
