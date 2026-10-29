defmodule CoreWeb.Problem do
  @moduledoc """
  Respuestas de error con Problem Details (RFC 9457), según la sección 14 de
  `docs/STANDARD_DEVELOPMENT.md`. `type` es una referencia relativa
  `/problems/<nombre>`, igual que en `contracts/openapi/auth.yaml`.

  Las respuestas no incluyen trazas, SQL, rutas internas ni secretos.
  """
  import Plug.Conn

  @doc "Envía un problema y detiene la conexión."
  def send_problem(conn, status, name, title, opts \\ []) do
    body =
      %{type: "/problems/#{name}", title: title, status: status}
      |> maybe_put(:detail, opts[:detail])
      |> maybe_put(:errors, opts[:errors])

    conn
    |> put_resp_content_type("application/problem+json")
    |> send_resp(status, Jason.encode_to_iodata!(body))
    |> halt()
  end

  @doc "422 con los errores de un changeset por campo."
  def send_validation_problem(conn, %Ecto.Changeset{} = changeset) do
    send_problem(conn, 422, "validation-error", "Invalid request",
      detail: "One or more fields are invalid.",
      errors: changeset_errors(changeset)
    )
  end

  @doc "Errores de un changeset como mapa `campo => [mensajes]`."
  def changeset_errors(changeset) do
    Ecto.Changeset.traverse_errors(changeset, fn {message, opts} ->
      Regex.replace(~r"%{(\w+)}", message, fn _, key ->
        opts |> Keyword.get(String.to_existing_atom(key), key) |> to_string()
      end)
    end)
  end

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
end
