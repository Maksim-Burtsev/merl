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
#         ^ d: app/brazier.rb:2
Brazier.make
#       ^ d: app/brazier.rb:2
Brazier.make.light
#            ^ d: app/brazier.rb:10
Grate.make.light
#          ^ d: app/brazier.rb:18
Grate::make
#      ^ d: app/brazier.rb:14
