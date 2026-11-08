defmodule Shop.Kwcafe do
  def mugful, do: 3

  def kwbrew(opts \\ []), do: Keyword.get(opts, :mugful, 0)

  def serve do
    mugful = 5
    kwbrew(mugful: mugful)
    #      ^ d: none
    #              ^ d: lib/shop/kwcafe.ex:7
    Shop.Kwcafe.kwbrew(mugful: 4)
    #                  ^ d: none
    kwbrew mugful: 4
    #      ^ d: none
  end
end

defmodule Shop.KwcafeNeighbours do
  defstruct mugful: 1

  def m(%Shop.KwcafeNeighbours{mugful: mugful}), do: mugful
  #                            ^ d: !jump
  def k(mugful), do: [mugful: mugful, b: 1]
  #                     ^ d: none
  #                             ^ d: !jump
end
