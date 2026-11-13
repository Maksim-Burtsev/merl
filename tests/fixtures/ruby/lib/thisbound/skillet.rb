class Skillet
  def fry
    3
  end

  def aliased
    me = self
    [1].each { me.fry }
    #             ^ d: lib/thisbound/skillet.rb:2
    me.fry
    #  ^ d: lib/thisbound/skillet.rb:2
    [Pan.new].each { |me| me.fry }
    #                        ^ d: !jump
    griddle = Griddle.new
    griddle.instance_eval do
      it = self
      it.fry
      #  ^ d: !jump
    end
  end

  def self.build
    klass = self
    klass.fry
    #     ^ d: !jump
  end
end
