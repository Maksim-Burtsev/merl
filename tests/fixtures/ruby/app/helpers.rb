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
