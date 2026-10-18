# The adversarial corner of #982 for Ruby, in a tea set's names.
module Tea
  module Pourable
    def pour(cups)
      cups
    end

    def spill
      pour(0)
    end
  end

  module Fillable
    def fill
      1
    end
  end

  class Vessel
    attr_accessor :volume

    def initialize(volume)
      @volume = volume
    end

    def self.empty
      new(0)
    end

    def steep
      volume
    end
  end
end
