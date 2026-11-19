defmodule CoreWeb.HealthController do
  @moduledoc """
  Verificación de estado del servicio (RNF-DIS-02).

  Responde 200 si Core puede consultar PostgreSQL y 503 si no. La respuesta no
  incluye detalles del error (RNF-SEG-04).
  """
  use CoreWeb, :controller

  def show(conn, _params) do
    case Ecto.Adapters.SQL.query(Core.Repo, "SELECT 1", []) do
      {:ok, _result} ->
        json(conn, %{status: "ok"})

      {:error, _reason} ->
        conn
        |> put_status(:service_unavailable)
        |> json(%{status: "unavailable"})
    end
  end
end
