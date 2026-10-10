# Ruby's locals (#383): each name below is also a method of `Rigging`, so a binding the scope walk
# loses turns the jump into the search by name.
module Shop
  class Rigging
    def jib
    end

    def bowsprit
    end

    def halyard
    end

    def scupper
    end

    def sheave
    end

    def lanyard
    end

    def haul(ready)
      if ready
        jib = 1
      end
      jib
    # ^ d: lib/shop/rigging.rb:25
    end

    def furl
      bowsprit = stow(
        1,
    )
      bowsprit
    # ^ d: picker lib/shop/rigging.rb:8, lib/shop/rigging.rb:32
    end

    def coil
      halyard = 2
      halyard
    # ^ d: lib/shop/rigging.rb:40
    end

    def tie(x) = x +
      halyard
    # ^ d: picker lib/shop/rigging.rb:11, lib/shop/rigging.rb:40

    def splice(ropes)
      scupper = 0
      ropes.each do |strand; scupper|
        scupper
      # ^ d: lib/shop/rigging.rb:51
      end
    end

    def reeve(sheave,
              block)
      sheave
    # ^ d: lib/shop/rigging.rb:57
    end

    def belay lanyard, count
      lanyard
    # ^ d: lib/shop/rigging.rb:63
    end

    def transom fore; jib
      jib
    # ^ d: lib/shop/rigging.rb:5
    end

    def lash fore, aft # fore, scupper
      scupper
    # ^ d: lib/shop/rigging.rb:14
    end

    def trim(text, pat = /#/, sheave = 1)
      sheave
    # ^ d: lib/shop/rigging.rb:78
    end

    def knot(text, pat = %r{#}, halyard = 1)
      halyard
    # ^ d: lib/shop/rigging.rb:83
    end
  end
end
