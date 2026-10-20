# Project patches of String that `d` must not call builtin: a module included or prepended into
# String, a `String.class_eval` block, a `refine(String)` block and a reopened `class String`, beside
# a namesake in an unrelated class that does not hide the core method.
module Tea
  module Swapping
    def swapcase
      self
    end
  end
  String.include Swapping

  module Squeezing
    def squeeze
      self
    end
  end
  String.prepend(Squeezing)

  String.class_eval do
    def center
      self
    end
  end

  module Padding
    refine(String) do
      def ljust
        self
      end
    end
  end

  class Glaze
    def chop
      self
    end

    def coat(raw)
      raw.to_s.swapcase + raw.to_s.squeeze + raw.to_s.center + raw.to_s.ljust + raw.to_s.rjust
      #        ^ d: picker lib/tea/glaze.rb:6
      #                            ^ d: picker lib/tea/glaze.rb:13
      #                                               ^ d: picker lib/tea/glaze.rb:20
      #                                                                 ^ d: picker lib/tea/glaze.rb:27
      #                                                                                  ^ d: picker lib/tea/glaze.rb:55
    end

    def thin(raw)
      raw.to_s.chop
      #        ^ d: none
    end
  end
end

class String
  def rjust
    self
  end
end
