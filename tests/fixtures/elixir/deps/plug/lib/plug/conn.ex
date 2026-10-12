defmodule Plug.Conn do
  def assign(conn, key, value) do
    {conn, key, value}
  end

  def merge(conn, key, value) do
    {conn, key, value}
  end
end
