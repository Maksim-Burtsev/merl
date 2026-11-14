defmodule Shop.Alchemy do
  alias Shop.Retort, as: Crucible

  def run do
    Crucible.distil()
  # ^ d: lib/shop/retort.ex:1
    #        ^ d: lib/shop/retort.ex:2
  end

  def plain do
    {Crucible}
   # ^ d: lib/shop/retort.ex:1
  end
end
