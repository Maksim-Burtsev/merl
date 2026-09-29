defmodule Shop.Ledger do
  import Shop.Pricing

  @rate_cap 5

  def cap, do: @rate_cap
  #              ^ d: picker lib/shop/ledger.ex:4, lib/shop/pricing.ex:2; want lib/shop/ledger.ex:4 (#460)

  def total(n), do: discount(n) + cents(n)
  #                 ^ d: picker lib/shop/pricing.ex:22, lib/shop/pricing.ex:23
  #                                ^ d: lib/shop/pricing.ex:31
end

defimpl Shop.Pricing.Priced, for: Shop.Basket do
  #                  ^ d: lib/shop/pricing.ex:18
  #                                    ^ d: lib/shop/basket.ex:1
  def price(basket), do: Shop.Basket.gross(basket)
  #                                  ^ d: lib/shop/basket.ex:27
end
