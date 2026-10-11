module Shop
  class Cart < Base
    def pay
      checkout
      # ^ d: app/shop_base.rb:5
    end
  end
end
