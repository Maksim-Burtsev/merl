module Shop
  class Satchel
    def initialize(grams:, owner: nil)
      @grams = grams
    end
  end

  def self.satchel
    Satchel.new(grams: 3, owner: "x")
    #                     ^ d: lib/shop/labels.rb:3
    h = { grams: 3 }
    #     ^ d: none
    #     status: grams: key
    h
  end
end
