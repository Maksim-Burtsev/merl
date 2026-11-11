def feast
  c = Kylix.new
  c.libate
  # ^ d: picker lib/kylix/kylix.rb:1, lib/kylix/vessels.rb:1, lib/kylix/vessels.rb:2, lib/kylix/vessels.rb:4; want lib/kylix/kylix.rb:1 (#1486)
  s = Skyphos.new
  s.libate
  # ^ d: picker lib/kylix/kylix.rb:1, lib/kylix/vessels.rb:1, lib/kylix/vessels.rb:2, lib/kylix/vessels.rb:4; want lib/kylix/vessels.rb:1 (#1486)
  r = Rhyton.new
  r.libate
  # ^ d: picker lib/kylix/kylix.rb:1, lib/kylix/vessels.rb:1, lib/kylix/vessels.rb:2, lib/kylix/vessels.rb:4; want lib/kylix/vessels.rb:2 (#1486)
  k = Kantharos.new
  k.libate
  # ^ d: lib/kylix/vessels.rb:4
end
