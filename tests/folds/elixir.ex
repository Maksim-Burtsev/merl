defmodule Shop.Cart do  # f: 1-61
  @moduledoc """
  A cart: do this, end that.
  """

  use Shop.Web, :controller

  plug :fetch_session,  # f: 8-9
    keys: [:cart, :user]

  defstruct [  # f: 11-14
    items: [],
    total: 0
  ]

  # end do fn ->
  def add(cart, item) do  # f: 17-19
    %{cart | items: [item | cart.items]}  # f: 17-19
  end

  def total(%__MODULE__{  # f: 21-22
        items: items
      }) do  # f: 23-27
    Enum.reduce(items, 0, fn %{price: p}, acc ->  # f: 24-25
      acc + p  # f: 23-27
    end)
  end

  def label(item) when is_map(item),  # f: 29-30
    do: "#{item.name} {#{item.price}}"

  def pick(x) do  # f: 32-42
    case x do  # f: 33-41
      {:ok,  # f: 34-35
       value} ->
        value  # f: 32-42

      :error when is_atom(x) ->  # f: 38-40
        ~s(end)
        # nothing left
    end
  end

  def run(x) do  # f: 44-53
    with {:ok, a} <- fetch(x),
         {:ok, b} <- check(a) do  # f: 46-52
      {a, b}
    else
      {:error, reason} ->  # f: 49-51
        IO.puts("do")  # f: 44-53
        reason
    end
  end

  defp tags, do: ~w[do end fn]a
  defp brace, do: ?{
  defp sum(xs), do: (  # f: 57-60
    xs  # f: 1-61
    |> Enum.sum()
  )
end
