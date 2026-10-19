# A bare call the class and its ancestors do not define is offered, not jumped to, when every
# method of its name sits in an unrelated class or is a `def self.` of another module; a concern
# calling its host class, a module's instance method, a top-level method and a schema column
# still jump (#1093).
class LinkExtractor
  def head
    nil
  end
end

module Untangling
  def self.untangle(text)
    text
  end
end

module Dusting
  def dust
    nil
  end
end

class PingsController < ApplicationController
  def create
    head :ok
    # ^ d: picker app/bare_calls.rb:6
    untangle(1)
    # ^ d: picker app/bare_calls.rb:12
    dust
    # ^ d: app/bare_calls.rb:18
    polish
    # ^ d: app/bare_calls.rb:55
    language
    # ^ d: db/schema.rb:4
  end
end

module Sweeping
  def sweep
    tidy
    # ^ d: app/bare_calls.rb:50
    head
    # ^ d: picker app/bare_calls.rb:6
  end
end

class Broom
  include Sweeping

  def tidy
    nil
  end
end

def polish
  nil
end

module Attaching
  extend ActiveSupport::Concern

  included do
    def self.attach_file(name)
      name
    end
  end
end

module Framing
  extend ActiveSupport::Concern

  included do
    attach_file :frame
    # ^ d: app/bare_calls.rb:63
  end
end

class Gallery
  include Attaching
  include Framing
end
