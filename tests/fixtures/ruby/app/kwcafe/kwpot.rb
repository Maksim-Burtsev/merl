class Kwpot
  attr_reader :mugful

  def initialize
    @mugful = 1
  end

  def kwbrew(mugful: 2)
    mugful
  end
end
