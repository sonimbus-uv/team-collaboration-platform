defmodule Core.ScyllaHelpers do
  @moduledoc """
  Las pruebas con la etiqueta `:scylla` usan el keyspace real
  `sonimbus_core` (ScyllaDB no tiene sandbox). Cada prueba crea canales con
  UUID nuevos, así que sus particiones no se mezclan. Sin ScyllaDB:
  `mix test --exclude scylla`.
  """

  @doc "Espera hasta 20 s a que Xandra tenga conexión."
  def await_scylla!(attempts \\ 40) do
    case Xandra.Cluster.execute(Core.Scylla, "SELECT release_version FROM system.local") do
      {:ok, _} ->
        :ok

      {:error, _} when attempts > 0 ->
        Process.sleep(500)
        await_scylla!(attempts - 1)

      {:error, error} ->
        raise "ScyllaDB no responde: #{inspect(error)}"
    end
  end
end
