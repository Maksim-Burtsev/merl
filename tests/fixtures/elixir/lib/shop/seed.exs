defmodule Shop.Seed do
  def seed_parcels(n), do: n
end

seed_parcels(3)
Shop.Seed.seed_parcels(3)
#         ^ d: lib/shop/seed.exs:2
seed_parcels(4)
4 |> seed_parcels()
#    ^ d: lib/shop/seed.exs:2
