defmodule Shop.Export do
  def dump(items), do: Enum.map(items, &Jason.encode!/1)
  #                         ^ d: none
  #                                           ^ d: deps/jason/lib/jason.ex:2

  def pack(basket), do: Jason.Encoder.encode(basket, [])
  #                                   ^ d: deps/jason/lib/jason/encoder.ex:2

  def show(socket, basket), do: Phoenix.LiveView.assign(socket, :basket, basket)
  #                                              ^ d: deps/phoenix_live_view/lib/phoenix_live_view.ex:2

  # `Phoenix.LiveView` does not declare it: another file of its package does, as a `use` injects.
  def notify(socket), do: Phoenix.LiveView.push_event(socket, "saved", %{})
  #                                        ^ d: deps/phoenix_live_view/lib/phoenix_live_view/utils.ex:2
  # status: push_event → Utils.push_event (by name, 1 match)

  def tag(conn, basket), do: Plug.Conn.assign(conn, :basket, basket)
  #                                    ^ d: deps/plug/lib/plug/conn.ex:2
end
