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

class Bellows
  def self.pump(air)
    new(air).blow(true)
  end

  def blow(hard)
    Brazier.new
  end

  def light; end
end

Bellows.pump(1).light
#               ^ d: !jump
