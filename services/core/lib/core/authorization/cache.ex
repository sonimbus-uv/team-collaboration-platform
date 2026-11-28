defmodule Core.Authorization.Cache do
  use GenServer

  @table __MODULE__
  @ttl_ms 60_000

  def start_link(_opts), do: GenServer.start_link(__MODULE__, nil, name: __MODULE__)

  def fetch(key) do
    now = System.monotonic_time(:millisecond)

    case :ets.lookup(@table, key) do
      [{^key, value, expires_at}] when expires_at > now -> {:ok, value}
      _ -> :miss
    end
  end

  def put(key, value) do
    :ets.insert(@table, {key, value, System.monotonic_time(:millisecond) + @ttl_ms})
    value
  end

  def delete_group(group_id) do
    :ets.match_delete(@table, {{group_id, :_}, :_, :_})
    :ok
  end

  @impl true
  def init(nil) do
    :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
    {:ok, nil}
  end
end
