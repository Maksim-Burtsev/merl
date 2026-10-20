# The third corner of #1126 for Ruby: a builder, namesakes of core methods, a refinement.
require_relative "kettle"

module Tea
  class Strainer
    def strip
      1
    end

    def dig(key)
      key
    end

    def chomp
      []
    end

    def upcase
      ""
    end
  end

  class Cup
    attr_reader :size, :next_cup

    def initialize(size)
      @size = size
      @next_cup = nil
    end

    def sip
      size
    end
  end

  class Brewer
    def leaves(n)
      @leaves = n
      self
    end

    def temperature(degrees)
      @degrees = degrees
      self
    end

    def build
      Cup.new(@leaves)
    end
  end

  class Infuser
    def leaves(n)
      n
    end

    def temperature(degrees)
      degrees
    end

    def build
      0
    end

    def sip
      0
    end
  end

  module Polish
    refine String do
      def shine
        upcase
      end
    end
  end

  class Samovar < Vessel
    include Pourable

    def pour(cups)
      super(cups) + 1
      # ^ d: none; want lib/tea/vessel.rb:4 (#1163)
    end
    alias_method :decant, :pour
  end

  Urn = Samovar
  Leaf = Struct.new(:color)

  module Kit
    module_function

    def make
      Brewer.new
    end
  end
end
