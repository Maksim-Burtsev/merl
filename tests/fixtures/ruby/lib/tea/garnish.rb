def garnish
  0
end

class Sprig
  def garnish
    5
  end

  def plate
    garnish
    # ^ d: lib/tea/garnish.rb:6
  end
end

def plate_top
  garnish
  # ^ d: lib/tea/garnish.rb:1
end

def plate_in_block
  [1].each { garnish }
  #          ^ d: picker lib/tea/garnish.rb:1, lib/tea/garnish.rb:6
end

def plate_do
  [1].each do
    garnish
    # ^ d: picker lib/tea/garnish.rb:1, lib/tea/garnish.rb:6
  end
end

def Sprig.dollop
  1
end

class Sprig
  def dollop
    2
  end
end

def plate_dollop
  dollop
  # ^ d: picker lib/tea/garnish.rb:33, lib/tea/garnish.rb:38
end

def tuck
  0
end

class Object
  def tuck
    1
  end
end

class Peel
  def tuck
    2
  end
end

def plate_tuck
  tuck
  # ^ d: picker lib/tea/garnish.rb:48, lib/tea/garnish.rb:53
end

def rind
  0
end

module Spice
  def rind
    1
  end
end

class Peel
  def rind
    2
  end
end

def plate_rind
  rind
  # ^ d: picker lib/tea/garnish.rb:69, lib/tea/garnish.rb:74
end
