defmodule Shop.Export do
  def dump(items), do: Enum.map(items, &Jason.encode!/1)
  #                         ^ d: none
  #                                           ^ d: deps/jason/lib/jason.ex:2

  def pack(basket), do: Jason.Encoder.encode(basket, [])
  #                                   ^ d: deps/jason/lib/jason/encoder.ex:2

  def show(socket, basket), do: Phoenix.LiveView.assign(socket, :basket, basket)
  #                                              ^ d: deps/phoenix_live_view/lib/phoenix_live_view.ex:2

  def tag(conn, basket), do: Plug.Conn.assign(conn, :basket, basket)
  #                                    ^ d: deps/plug/lib/plug/conn.ex:2
end
