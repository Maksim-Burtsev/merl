module Helpers
  module_function

  def slugify(text)
    text
  end
end

module Paths
  extend self

  def root
    "/"
  end
end

class Clock
  def self.tick
    1
  end

  def tick
    2
  end
end

module Stamps
  def stamp(x)
    x
  end

  def seal(x)
    x
  end
  module_function :stamp, :seal
end
