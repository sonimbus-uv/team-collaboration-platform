defmodule CoreWeb.Plugs.Authenticate do
  @moduledoc """
  Exige un access JWT de Auth en `Authorization: Bearer <token>` y lo verifica
  con `Core.AccessToken`. Asigna `current_user_id` (`sub`),
  `current_session_id` (`sid`) y `token_expires_at` (`exp`).

  - Token ausente o inválido: 401 con `WWW-Authenticate: Bearer`.
  - JWKS no disponible: 503; un fallo de red no demuestra que el token sea
    inválido (`contracts/token-profile.md`).

  No registra el encabezado ni el token.
  """
  @behaviour Plug

  import Plug.Conn

  alias Core.AccessToken
  alias CoreWeb.Problem

  @impl true
  def init(opts), do: opts

  @impl true
  def call(conn, _opts) do
    with {:ok, token} <- bearer_token(conn),
         {:ok, claims} <- AccessToken.verify(token) do
      conn
      |> assign(:current_user_id, claims.user_id)
      |> assign(:current_session_id, claims.session_id)
      |> assign(:token_expires_at, claims.expires_at)
    else
      {:error, :unavailable} ->
        conn
        |> put_resp_header("retry-after", "5")
        |> Problem.send_problem(503, "auth-unavailable", "Authentication unavailable",
          detail: "The access token cannot be verified right now."
        )

      _ ->
        conn
        |> put_resp_header("www-authenticate", ~s(Bearer error="invalid_token"))
        |> Problem.send_problem(401, "unauthorized", "Unauthorized",
          detail: "A valid access token is required."
        )
    end
  end

  defp bearer_token(conn) do
    with [header] <- get_req_header(conn, "authorization"),
         [scheme, token] when token != "" <- String.split(header, " ", parts: 2),
         "bearer" <- String.downcase(scheme) do
      {:ok, token}
    else
      _ -> :error
    end
  end
end
