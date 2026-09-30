defmodule Phoenix.LiveView do
  def assign(socket, key, value) do
    {socket, key, value}
  end
end
