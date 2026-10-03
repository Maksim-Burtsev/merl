defmodule Shop.Report do
  def summary(id), do: :shop_parcel.grams(:shop_parcel.make(id))
  #                                 ^ d: picker src/shop_parcel.erl:33, src/shop_parcel.erl:37
  #                                                    ^ d: src/shop_parcel.erl:25
  #                     ^ d: src/shop_parcel.erl:1

  def listen, do: :ranch.start_listener(:shop, :tcp, %{})
  #                      ^ d: deps/ranch/src/ranch.erl:4

  def again, do: stamp()
  #              ^ d: none
end
