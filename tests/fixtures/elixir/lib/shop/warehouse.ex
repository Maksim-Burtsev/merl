defmodule Shop.Warehouse do
  defmodule Courier do
    defstruct name: "post", slots: 1

    def new(name), do: %Courier{name: name}
  end

  def weigh(grams) when is_integer(grams), do: div(grams, 1000)
  def weigh(_grams), do: 0

  defdelegate depot(name), to: Shop.Warehouse.Depot, as: :open

  def full?(courier), do: courier.slots == 0
  def ship!(courier), do: courier
end

defmodule Shop.Warehouse.Depot do
  def open(name), do: name
end

defimpl Shop.Pricing.Priced, for: Shop.Warehouse.Courier do
  def price(_courier), do: 5
end
