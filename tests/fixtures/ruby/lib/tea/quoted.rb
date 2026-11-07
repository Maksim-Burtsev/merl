module Brewable
  def ladle
  end

  def whisk
  end

  def simmer
  end

  def brine
  end

  def swirl
  end

  def zest
  end
end

class Mug
  include Brewable

  TEMPLATE = %q{
    def ladle
    end
  }
  BIG = %Q(
    def whisk
    end
  )
  BARE = %(
    def simmer
    end
  )
  LIST = %w[
    def brine
  ]
  TEXT = "a \\"
  TOLD = "
    def swirl
    end
  "
  ONE = 'it\'s
    def zest
  '

  def go
    ladle
#   ^ d: lib/tea/quoted.rb:2
    whisk
#   ^ d: lib/tea/quoted.rb:5
    simmer
#   ^ d: lib/tea/quoted.rb:8
    brine
#   ^ d: lib/tea/quoted.rb:11
    swirl
#   ^ d: lib/tea/quoted.rb:14
    zest
#   ^ d: lib/tea/quoted.rb:17
    mellow
#   ^ d: lib/tea/quoted.rb:67
  end

  CHAR = ?"
  RULE = { error: /can't be blank/ }
  def mellow
  end
end
