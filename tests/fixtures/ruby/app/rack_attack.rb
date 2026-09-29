class Rack::Attack
  Rack::Attack.blocklist('deny') { |req| req }
# ^ d: none
  #     ^ d: app/rack_attack.rb:1
  #            ^ d: none
end
