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
