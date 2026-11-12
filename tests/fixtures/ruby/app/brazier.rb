class Brazier
  def self.make
    new
  end

  def make(size)
    size
  end

  def light; end
end

class Grate
  def self.make
    new
  end

  def light; end
end

Brazier::make
#         ^ d: picker app/brazier.rb:2, app/brazier.rb:6; want app/brazier.rb:2 (#1489)
Brazier.make
#       ^ d: app/brazier.rb:2
Brazier.make.light
#            ^ d: picker app/brazier.rb:10, app/brazier.rb:18; want app/brazier.rb:10 (#1489)
