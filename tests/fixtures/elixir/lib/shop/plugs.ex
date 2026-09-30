# #459: an `alias` renames a module below it, inside the block it is written in only.
defmodule Conn do
  def assign(c), do: c
end

defmodule Shop.Plugged do
  alias Plug.Conn

  def put(c), do: Conn.assign(c, :k, 1)
  #                    ^ d: deps/plug/lib/plug/conn.ex:2
end

defmodule Shop.Unplugged do
  def put(c), do: Conn.assign(c)
  #                    ^ d: lib/shop/plugs.ex:3
end
