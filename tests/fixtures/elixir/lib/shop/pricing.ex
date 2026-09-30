defmodule Shop.Pricing do
  @rate_cap 100

  defmodule Tariff do
    defstruct [:base, :zone]

    def rate(%Tariff{base: base}), do: base
    def describe(_t), do: "tariff"
  end

  defmodule Coupon do
    defstruct code: nil, off: 2

    def rate(%Coupon{off: off}), do: off
    def describe(_c), do: "coupon"
  end

  defprotocol Priced do
    def price(item)
  end

  def discount(total) when total > @rate_cap, do: @rate_cap - 1
  def discount(total), do: total - 1

  defp round_down(total), do: div(total, 10) * 10

  def settle(total) do
    round_down(total)
  end

  defmacro cents(x) do
    quote do: unquote(x) * 100
  end

  defguard is_cheap(total) when total < 10

  @doc ~S"""
  A sigil heredoc declares nothing either:
      def discount(total) do
  """
  def banner, do: "shop"
end
