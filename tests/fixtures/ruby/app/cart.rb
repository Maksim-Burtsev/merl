module Shop
  class Cart < Base
    def pay
      checkout
      # ^ d: app/shop_base.rb:5
      @basket
      # ^ d: app/shop_base.rb:8
    end
  end
end
