class Stylus
  def blot
    "stylus"
  end
end

class Nib
  def blot
    "nib"
  end
end

def quoted_a
  s = Stylus.new
  "s.blot #{s.blot}" + 's.blot'
  #   ^ d: picker lib/shop/quoted.rb:2, lib/shop/quoted.rb:8
  #            ^ d: lib/shop/quoted.rb:2
  #                        ^ d: picker lib/shop/quoted.rb:2, lib/shop/quoted.rb:8
end
