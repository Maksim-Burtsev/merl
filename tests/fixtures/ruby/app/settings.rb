class Store
  def cache
    {}
  end
end

STORE = Store.new

module Relays
end

class Settings
  def self.motto = "settings"
  #        ^ d: app/settings.rb:13

  def warm
    STORE.cache
    #     ^ d: app/settings.rb:2
  end
end

class Api::Relays::Hub
  #        ^ d: picker app/settings.rb:9
end
