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
  #         ^ d: picker lib/shop/kwcafe.ex:2

  def m(%Shop.KwcafeNeighbours{mugful: mugful}), do: mugful
  #                            ^ d: picker lib/shop/kwcafe.ex:2, lib/shop/kwcafe.ex:19
  def k(mugful), do: [mugful: mugful, b: 1]
  #                     ^ d: none
  #                             ^ d: !jump
end

defmodule Shop.KwcafeImports do
  import Shop.Kwcafe, only: [kwbrew: 1]
  #                          ^ d: lib/shop/kwcafe.ex:4
  import Shop.Kwcafe, except: [serve: 0]
  #                            ^ d: lib/shop/kwcafe.ex:6
  def kwsteep(n), do: n
  defoverridable kwsteep: 1
  #              ^ d: lib/shop/kwcafe.ex:34
  defoverridable [kwsteep: 1]
  #               ^ d: lib/shop/kwcafe.ex:34
  # serve: the pot, then kwsteep(mugful: 4)
  # ^ d: lib/shop/kwcafe.ex:6
  #                              ^ d: picker lib/shop/kwcafe.ex:2, lib/shop/kwcafe.ex:19
  def note, do: "serve: hot"
  #              ^ d: lib/shop/kwcafe.ex:6
  @doc """
  serve: hot
  # ^ d: lib/shop/kwcafe.ex:6
  """
  def pour, do: :ok
end
