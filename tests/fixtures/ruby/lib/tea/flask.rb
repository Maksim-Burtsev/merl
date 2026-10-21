# `new` lands on the `initialize` Ruby calls: the class's, an included module's, then up the superclasses (#1185).
module Tea
  module Warming
    def initialize(heat)
      @heat = heat
    end
  end

  class Flask
    include Warming
  end

  class Thermos < Vessel
    def self.new(volume)
      super(volume)
    end
  end

  class Jug < ::Outside::Base
  end

  class Carafe < Samovar
    include Fillable
  end

  module Shelf
    def self.stock
      Flask.new(1).to_s + Thermos.new(1).to_s + Jug.new.to_s + Carafe.new(1).to_s
      #     ^ d: lib/tea/flask.rb:4
      #                           ^ d: none
      #                                             ^ d: none
      #                                                               ^ d: lib/tea/vessel.rb:22
    end
  end
end
