class Gridiron
  def sear
    2
  end

  def scoped
    griddle = Griddle.new
    griddle.instance_eval { sear }
    #                       ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    griddle.instance_exec { sear }
    #                       ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    griddle.instance_eval { self.sear }
    #                            ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    griddle.instance_eval do
      sear
    # ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    end
    [1].each { sear }
    #          ^ d: lib/thisbound/gridiron.rb:2
    [1].each { self.sear }
    #               ^ d: lib/thisbound/gridiron.rb:2
    Griddle.new.instance_eval { sear }
    #                           ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    Griddle.class_eval { sear }
    #                    ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    Griddle.module_eval { sear }
    #                     ^ d: picker lib/thisbound/gridiron.rb:2, lib/thisbound/griddle.rb:2
    griddle.tap { sear }
    #             ^ d: lib/thisbound/gridiron.rb:2
  end
end
