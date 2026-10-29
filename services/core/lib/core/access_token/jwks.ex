defmodule Core.AccessToken.JWKS do
  @moduledoc """
  Caché del JWKS de Auth (`contracts/token-profile.md`, «JWKS y ciclo de
  claves»).

  - Vigencia máxima de 5 minutos desde la última descarga correcta; no se
    renueva con el uso.
  - Con el JWKS vencido, si la descarga falla no se usan las claves viejas:
    `{:error, :unavailable}` (la API responde 503).
  - Un `kid` desconocido provoca a lo sumo una descarga cada
    `:min_refresh_ms`; las peticiones simultáneas esperan la misma descarga
    porque pasan por este proceso. Si tras descargar sigue sin aparecer,
    `{:error, :unknown_kid}` (401).

  Las lecturas con caché vigente van directo a ETS, sin pasar por el proceso.
  """
  use GenServer

  require Logger

  @default_max_age_ms 300_000
  @default_min_refresh_ms 10_000
  @call_timeout 5_000

  ## API

  def start_link(opts) do
    name = Keyword.get(opts, :name, __MODULE__)
    GenServer.start_link(__MODULE__, Keyword.put(opts, :name, name), name: name)
  end

  @doc "Clave pública Ed25519 (32 bytes) para `kid`."
  @spec get_key(GenServer.server(), binary()) ::
          {:ok, binary()} | {:error, :unknown_kid | :unavailable}
  def get_key(server \\ __MODULE__, kid) do
    case lookup_fresh(server) do
      {:ok, %{^kid => key}} -> {:ok, key}
      _ -> call(server, {:get_key, kid})
    end
  end

  @doc """
  Convierte el cuerpo de un JWKS en un mapa `kid => clave pública`. Solo
  acepta claves `OKP`/`Ed25519` de 32 bytes, sin `d`; `use` y `alg`, si
  vienen, deben ser `sig` y `EdDSA`. Un `kid` repetido invalida el conjunto.
  """
  @spec parse(binary()) :: {:ok, %{binary() => binary()}} | :error
  def parse(body) do
    with {:ok, %{"keys" => [_ | _] = keys}} <- Jason.decode(body),
         parsed = Enum.map(keys, &parse_key/1),
         kids = Enum.map(parsed, &elem(&1, 0)),
         true <- length(kids) == length(Enum.uniq(kids)),
         valid = for({kid, {:ok, key}} <- parsed, into: %{}, do: {kid, key}),
         true <- map_size(valid) > 0 do
      {:ok, valid}
    else
      _ -> :error
    end
  end

  defp parse_key(%{"kid" => kid} = jwk) when is_binary(kid) and kid != "" do
    {kid, ed25519_key(jwk)}
  end

  defp parse_key(_jwk), do: {make_ref(), :error}

  defp ed25519_key(%{"kty" => "OKP", "crv" => "Ed25519", "x" => x} = jwk)
       when is_binary(x) do
    with false <- Map.has_key?(jwk, "d"),
         true <- Map.get(jwk, "use", "sig") == "sig",
         true <- Map.get(jwk, "alg", "EdDSA") == "EdDSA",
         {:ok, <<_::binary-size(32)>> = key} <- Base.url_decode64(x, padding: false) do
      {:ok, key}
    else
      _ -> :error
    end
  end

  defp ed25519_key(_jwk), do: :error

  ## Servidor

  @impl true
  def init(opts) do
    config = Application.get_env(:core, Core.AccessToken, [])
    table = :ets.new(table_name(opts[:name]), [:named_table, :protected, read_concurrency: true])

    state = %{
      table: table,
      fetcher: Keyword.get_lazy(opts, :fetcher, fn -> Keyword.fetch!(config, :jwks_fetcher) end),
      max_age_ms: Keyword.get(opts, :max_age_ms, @default_max_age_ms),
      min_refresh_ms: Keyword.get(opts, :min_refresh_ms, @default_min_refresh_ms),
      last_attempt: nil
    }

    :ets.insert(table, {:config, state.max_age_ms})
    {:ok, state}
  end

  @impl true
  def handle_call({:get_key, kid}, _from, state) do
    # Otra petición pudo haber descargado el JWKS mientras esta esperaba.
    case lookup_fresh_table(state.table) do
      {:ok, %{^kid => key}} ->
        {:reply, {:ok, key}, state}

      {:ok, _keys} ->
        if refresh_allowed?(state) do
          state = refresh(state)
          {:reply, find(state.table, kid, :unknown_kid), state}
        else
          {:reply, {:error, :unknown_kid}, state}
        end

      :stale ->
        if refresh_allowed?(state) do
          state = refresh(state)
          {:reply, find(state.table, kid, :unavailable), state}
        else
          {:reply, {:error, :unavailable}, state}
        end
    end
  end

  defp find(table, kid, missing_reason) do
    case lookup_fresh_table(table) do
      {:ok, %{^kid => key}} -> {:ok, key}
      {:ok, _keys} -> {:error, :unknown_kid}
      :stale -> {:error, missing_reason}
    end
  end

  defp refresh_allowed?(%{last_attempt: nil}), do: true

  defp refresh_allowed?(state),
    do: now_ms() - state.last_attempt >= state.min_refresh_ms

  # Un fallo no borra un JWKS todavía vigente: solo deja de renovarse.
  defp refresh(state) do
    {module, function, args} = state.fetcher

    result =
      with {:ok, body} <- apply(module, function, args),
           {:ok, keys} <- parse(body) do
        {:ok, keys}
      end

    case result do
      {:ok, keys} ->
        :ets.insert(state.table, {:keys, keys, now_ms()})

      _ ->
        Logger.warning("no se pudo actualizar el JWKS de Auth")
    end

    %{state | last_attempt: now_ms()}
  end

  ## Lectura

  defp lookup_fresh(server) when is_atom(server) do
    lookup_fresh_table(table_name(server))
  rescue
    ArgumentError -> :stale
  end

  defp lookup_fresh(_server), do: :stale

  defp lookup_fresh_table(table) do
    with [{:config, max_age_ms}] <- :ets.lookup(table, :config),
         [{:keys, keys, fetched_at}] <- :ets.lookup(table, :keys),
         true <- now_ms() - fetched_at < max_age_ms do
      {:ok, keys}
    else
      _ -> :stale
    end
  end

  defp call(server, message) do
    GenServer.call(server, message, @call_timeout)
  catch
    :exit, _reason -> {:error, :unavailable}
  end

  defp table_name(name) when is_atom(name), do: Module.concat(name, Table)

  defp now_ms, do: System.monotonic_time(:millisecond)
end
