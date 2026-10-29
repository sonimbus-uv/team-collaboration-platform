defmodule CoreWeb.Plugs.RequireUser do
  @moduledoc """
  Exige `conn.assigns.current_user_id`. Responde 401 con Problem Details si
  falta.
  """
  @behaviour Plug

  alias CoreWeb.Problem

  @impl true
  def init(opts), do: opts

  @impl true
  def call(%Plug.Conn{assigns: %{current_user_id: user_id}} = conn, _opts)
      when is_binary(user_id),
      do: conn

  def call(conn, _opts) do
    conn
    |> Plug.Conn.put_resp_header("www-authenticate", "Bearer")
    |> Problem.send_problem(401, "unauthorized", "Unauthorized",
      detail: "A valid access token is required."
    )
  end
end
