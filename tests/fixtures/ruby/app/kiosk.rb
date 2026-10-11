# `include ::Coupons` is the top-level module, never `Shop::Coupons` (#793).
module Shop
  module Coupons
    def headcount = 3
  end

  class Kiosk
    include ::Coupons

    def ring
      headcount
      # ^ d: app/coupons.rb:2
    end
  end
end
