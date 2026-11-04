defmodule CoreWeb.UserSocket do
  @moduledoc """
  WebSocket de los clientes (RealtimeService). La conexión exige el access
  JWT de Auth, verificado con `Core.AccessToken`:

  - preferido: `authToken` del cliente de Phoenix, que viaja en
    `Sec-WebSocket-Protocol` y no en la URL;
  - alternativa: parámetro `token`, filtrado de los registros.

  El token se verifica al conectar; cada canal se cierra cuando vence su
  `exp` (`CoreWeb.MessageChannel`) y el cliente debe reconectar con un token
  nuevo.
  """
  use Phoenix.Socket

  alias Core.AccessToken

  channel "channel:*", CoreWeb.MessageChannel

  @impl true
  def connect(params, socket, connect_info) do
    token = connect_info[:auth_token] || params["token"]

    with token when is_binary(token) <- token,
         {:ok, claims} <- AccessToken.verify(token) do
      {:ok,
       socket
       |> assign(:current_user_id, claims.user_id)
       |> assign(:token_expires_at, claims.expires_at)}
    else
      _ -> {:error, :unauthorized}
    end
  end

  # Permite desconectar todas las conexiones de un usuario con
  # CoreWeb.Endpoint.broadcast("user_socket:" <> user_id, "disconnect", %{}).
  @impl true
  def id(socket), do: "user_socket:#{socket.assigns.current_user_id}"
end
