defmodule Shop.Basket do
  alias Shop.Pricing.{Tariff, Coupon}
  #                   ^ d: lib/shop/pricing.ex:4
  #                           ^ d: lib/shop/pricing.ex:11
  alias Shop.Warehouse, as: W
  #          ^ d: lib/shop/warehouse.ex:1
  alias Shop.Warehouse.Courier
  import Shop.Pricing, only: [discount: 1, cents: 1]
  #           ^ d: lib/shop/pricing.ex:1
  require Shop.Pricing

  @weight_limit 30

  defstruct [:tariff, :coupon, owner: ""]

  @spec new(map) :: map
  #^ d: none
  #     ^ d: picker lib/shop/basket.ex:19, lib/shop/warehouse.ex:5; want lib/shop/basket.ex:19 (#460)
  def new(tariff) do
    %Shop.Basket{tariff: tariff, coupon: %Coupon{off: 3}}
    #     ^ d: lib/shop/basket.ex:1
    #            ^ d: lib/shop/basket.ex:14
    #                                     ^ d: lib/shop/pricing.ex:11
    #                                             ^ d: lib/shop/pricing.ex:12
  end

  def gross(%Shop.Basket{tariff: t}) do
    discount(Tariff.rate(t))
    # ^ d: picker lib/shop/pricing.ex:22, lib/shop/pricing.ex:23
    #               ^ d: lib/shop/pricing.ex:7
  end

  def bonus(basket) do
    Coupon.rate(basket.coupon) + cents(1)
    #       ^ d: lib/shop/pricing.ex:14
    #                  ^ d: lib/shop/basket.ex:14
    #                              ^ d: lib/shop/pricing.ex:31
  end

  def describe_any(t, c) do
    Tariff.describe(t) <> Coupon.describe(c)
    #       ^ d: lib/shop/pricing.ex:8
    #                              ^ d: lib/shop/pricing.ex:15
  end

  def heavy?(grams) do
    W.weigh(grams) > @weight_limit
    # ^ d: picker lib/shop/warehouse.ex:8, lib/shop/warehouse.ex:9
    #                 ^ d: lib/shop/basket.ex:12
  end

  def ship(courier) do
    if W.full?(courier), do: courier, else: W.ship!(courier)
    #    ^ d: lib/shop/warehouse.ex:13
    #                                         ^ d: lib/shop/warehouse.ex:14
  end

  def dispatch(n) do
    courier = Courier.new("post")
    #         ^ d: lib/shop/warehouse.ex:2
    #                 ^ d: lib/shop/warehouse.ex:5
    %Courier{courier | name: n}
    #                  ^ d: lib/shop/warehouse.ex:3
  end

  def depot, do: W.depot("north") && Shop.Warehouse.Depot.open("south")
  #                ^ d: lib/shop/warehouse.ex:11
  #                                                   ^ d: lib/shop/warehouse.ex:17
  #                                                         ^ d: lib/shop/warehouse.ex:18

  def pay(total) when Shop.Pricing.is_cheap(total) do
    #                              ^ d: lib/shop/pricing.ex:35
    Shop.Pricing.settle(total) + String.length(Shop.currency())
    #            ^ d: lib/shop/pricing.ex:27
    #                                                ^ d: lib/shop.ex:12
  end

  def price(courier) do
    Shop.Pricing.Priced.price(courier)
    #            ^ d: lib/shop/pricing.ex:18
    #                   ^ d: lib/shop/pricing.ex:19
  end

  def show, do: Shop.Pricing.banner()
  #                          ^ d: lib/shop/pricing.ex:41

  def restock(discount) do
    discount + 1
    # ^ d: picker lib/shop/pricing.ex:22, lib/shop/pricing.ex:23; want lib/shop/basket.ex:87 (#460)
  end

  def weigh_all(grams) do
    weigh = W.weigh(grams)
    weigh + 1
    # ^ d: picker lib/shop/warehouse.ex:8, lib/shop/warehouse.ex:9; want lib/shop/basket.ex:93 (#460)
  end

  def encode(basket), do: Jason.encode!(basket)
  #                             ^ d: deps/jason/lib/jason.ex:2

  def reship(courier), do: ship(courier)
  #                        ^ d: lib/shop/basket.ex:52
end
