defmodule Books.Ledger do
  @moduledoc """
  A ledger keeps entries in order.
  """

  use GenServer
  require Logger
  import Enum, only: [map: 2, sum: 1]
  alias Books.{Entry, Error}

  @limit 1_000
  @pattern ~r/[a-z]+\d/i
  @native ~Z"""
  pub fn twice(x: i64) i64 {
      return x * 2;
  }
  """

  defstruct owner: nil, entries: [], opened: nil

  @type t :: %__MODULE__{owner: String.t(), entries: [Entry.t()]}

  defmodule Entry do
    @enforce_keys [:id, :amount]
    defstruct [:id, :amount, note: nil]
  end

  @doc "An empty ledger."
  @spec empty() :: t()
  def empty, do: %__MODULE__{owner: "nobody"}

  def new(owner, opts \\ []) when is_binary(owner) and owner != "" do
    %__MODULE__{owner: owner, opened: Keyword.get(opts, :opened, DateTime.utc_now())}
  end

  def new(_owner, _opts), do: raise(ArgumentError, "owner is blank")

  def total(%__MODULE__{entries: entries}) do
    entries |> map(& &1.amount) |> sum()
  end

  def add(%__MODULE__{entries: entries} = ledger, amount, note \\ nil) do
    entry = %Entry{id: length(entries), amount: amount, note: note}
    {entry, %{ledger | entries: entries ++ [entry]}}
  end

  def sync(ledger, ids) do
    for id <- ids, id in 0..@limit do
      Logger.info("synced #{id} of #{length(ledger.entries)}")
    end
    :ok
  end

  def kind(%Entry{amount: amount}) do
    cond do
      amount < 0 -> :debit
      amount == 0 -> raise Error, message: "zero"
      true -> :credit
    end
  end

  def report(ledger) do
    lines =
      Enum.map(ledger.entries, fn e ->
        :io_lib.format("~-10s ~8.2f", [e.note || "-", abs(e.amount)])
      end)

    raw = """
    total: #{total(ledger)}
    count: #{length(ledger.entries)}
    """

    Enum.join(lines, "\n") <> raw
  end

  def check(x) do
    ch = ?a
    big = 0xFF_FF
    ratio = 1.5e3
    words = ~w(a b c)a
    is_binary(x) and byte_size(x) > 0 or ch == ?\n or (big > 0 and ratio != 0 and words != [])
  end

  @impl true
  def handle_call({:add, amount}, _from, state) do
    case add(state, amount) do
      {%Entry{} = entry, state} -> {:reply, {:ok, entry}, state}
      _ -> {:reply, :error, state}
    end
  end

  defp stale?(%{opened: nil}), do: false
  defp stale?(%{opened: opened}), do: DateTime.diff(DateTime.utc_now(), opened) > 3600

  def run do
    with {:ok, ledger} <- {:ok, empty()},
         {_, ledger} <- add(ledger, 10.0, "start") do
      ledger
    else
      _ -> nil
    end
  rescue
    e in Error -> Logger.error(Exception.message(e))
  after
    IO.puts("done")
  end
end
