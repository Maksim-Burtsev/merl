require_relative "vessel"

module Tea
  RECIPE = <<~RUBY
    def descale
    class Teapot
    def whistle
  RUBY

  class Kettle < Vessel
    include Pourable
    extend Fillable
    attr_accessor :heat

    def initialize(heat)
      super(1)
      @heat = heat
    end

    def self.cold
      new(0)
    end

    def cold
      heat < 50
    end

    define_method(:whistle) { heat > 99 }

    class << self
      def hot
        new(100)
      end
    end
  end

  class Teapot < Vessel
    include Pourable

    def pour(cups)
      cups + 1
    end
  end
end
