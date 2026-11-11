defmodule Shop.Foreign do
  alias Flask.Retort, as: Crucible

  def run, do: Crucible.distil()
  #            ^ d: none
  #                     ^ d: !jump

  def plain, do: {Crucible}
  #               ^ d: none

  def spelled, do: Flask.Retort.distil()
  #                             ^ d: !jump
end
