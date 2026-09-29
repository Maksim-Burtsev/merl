# The shop of #307: every `d` case carries its answer in a comment under it.
require_relative "pricing"
require_relative "warehouse"

module Shop
  WEIGHT_LIMIT = 30

  class Basket
    attr_reader :tariff, :coupon

    def initialize(tariff, coupon = Coupon.blank)
      #                             ^ d: lib/shop/pricing.rb:26
      #                                    ^ d: lib/shop/pricing.rb:38
      @tariff = tariff
      @coupon = coupon
    end

    def gross
      Shop.discount(@tariff.rate)
      #    ^ d: lib/shop/pricing.rb:54
      #                     ^ d: picker lib/shop/pricing.rb:17, lib/shop/pricing.rb:30
    end

    def bonus
      coupon.rate + gross
      #             ^ d: lib/shop/basket.rb:18
    end

    def describe_any(t, c)
      t.describe + c.caption + c.label
      # ^ d: picker lib/shop/pricing.rb:21, lib/shop/pricing.rb:34
      #              ^ d: picker lib/shop/pricing.rb:51
      #                          ^ d: picker lib/shop/pricing.rb:50
    end

    def restock(discount)
      discount + WEIGHT_LIMIT
      # ^ d: lib/shop/pricing.rb:54; want lib/shop/basket.rb:36 (#365)
      #          ^ d: lib/shop/basket.rb:6
    end

    def overweight(grams)
      limit = WEIGHT_LIMIT + 20
      Warehouse.weigh(grams) > limit
      # ^ d: lib/shop/warehouse.rb:2
      #         ^ d: lib/shop/warehouse.rb:22
      #                        ^ d: lib/shop/basket.rb:43
    end

    def hidden(grams)
      weigh = Warehouse.weigh(grams)
      weigh + Warehouse::LIMIT
      # ^ d: picker lib/shop/basket.rb:51, lib/shop/warehouse.rb:22; want lib/shop/basket.rb:51 (#365)
      #                  ^ d: lib/shop/warehouse.rb:3
    end

    def dispatch
      courier = Warehouse::Courier.new("post")
      #                    ^ d: lib/shop/warehouse.rb:5
      courier.name
      #       ^ d: picker lib/shop/warehouse.rb:6
    end

    def pick(couriers)
      couriers.first
      #        ^ d: picker lib/shop/warehouse.rb:12
      #        status: first: by name, 1 match
    end

    def check(coupon)
      coupon.expired? || coupon.expired!
      #      ^ d: picker lib/shop/pricing.rb:42
      #                         ^ d: picker lib/shop/pricing.rb:46
    end

    def stamp(coupon)
      coupon.code = "x"
      #      ^ d: picker lib/shop/pricing.rb:27
      coupon.owner = "me"
      #      ^ d: picker lib/shop/pricing.rb:28
      @tariff.base
      #       ^ d: picker lib/shop/pricing.rb:11
    end

    def depot
      Warehouse.open
      #         ^ d: lib/shop/warehouse.rb:27
    end

    def settle(total)
      Shop.settle(total)
      #    ^ d: lib/shop/pricing.rb:67
    end

    def audit(order)
      order.lines.recent
      #     ^ d: none; want lib/shop/order.rb:3 (#374)
      #           ^ d: none; want lib/shop/order.rb:4 (#374)
    end

    def ledger
      Ledger.new.prepare!
      #          ^ d: picker lib/shop/order.rb:19
    end

    def rated(courier)
      Warehouse::Courier.new(rate: 3)
      #                      ^ d: picker lib/shop/pricing.rb:17, lib/shop/pricing.rb:30; want none (#315)
    end

    def routed(courier)
      courier.route
      #       ^ d: none
    end

    def each_coupon(coupons)
      coupons.each do |coupon|
        coupon.describe
        # ^ d: lib/shop/basket.rb:9; want lib/shop/basket.rb:116 (#365)
      end
    end

    def rewrap
      @coupon.describe
      #^ d: lib/shop/basket.rb:15
    end
  end
end

