defmodule Core.Scylla do
  @moduledoc """
  Conexión a ScyllaDB con Xandra (D-05): keyspace `sonimbus_core`,
  consistencia `ONE` (un nodo en desarrollo y en la expo). Los esquemas viven
  en `database/scylla/`.

  Los errores del driver se reducen a `{:error, :unavailable}`; el detalle va
  al registro y nunca a la respuesta (RNF-SEG-04).
  """
  require Logger

  def child_spec(_opts) do
    config = Application.fetch_env!(:core, __MODULE__)

    Xandra.Cluster.child_spec(
      name: __MODULE__,
      nodes: Keyword.fetch!(config, :nodes),
      keyspace: Keyword.get(config, :keyspace, "sonimbus_core"),
      pool_size: Keyword.get(config, :pool_size, 5),
      default_consistency: :one
    )
  end

  @doc "Ejecuta una sentencia preparada; Xandra guarda las preparadas en caché."
  def execute(statement, params) do
    with {:ok, prepared} <- Xandra.Cluster.prepare(__MODULE__, statement),
         {:ok, result} <- Xandra.Cluster.execute(__MODULE__, prepared, params) do
      {:ok, result}
    else
      {:error, error} ->
        Logger.error("fallo de ScyllaDB: #{error_kind(error)}")
        {:error, :unavailable}
    end
  end

  defp error_kind(%Xandra.Error{reason: reason}), do: inspect(reason)
  defp error_kind(%Xandra.ConnectionError{action: action}), do: "conexión (#{action})"
  defp error_kind(%{__struct__: struct}), do: inspect(struct)
  defp error_kind(_error), do: "desconocido"

  @doc "Filas de un resultado como lista de mapas con claves de texto."
  def rows(%Xandra.Page{} = page), do: Enum.to_list(page)
  def rows(_result), do: []
end
