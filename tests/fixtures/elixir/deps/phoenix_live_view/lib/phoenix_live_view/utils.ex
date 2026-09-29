defmodule Phoenix.LiveView.Utils do
  def push_event(socket, event, payload) do
    {socket, event, payload}
  end
end
