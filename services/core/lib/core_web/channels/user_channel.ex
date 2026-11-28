defmodule CoreWeb.UserChannel do
  use CoreWeb, :channel

  alias CoreWeb.SocketToken

  @impl true
  def join("user:" <> user_id, _payload, socket) do
    with {:ok, ms_left} <- SocketToken.time_left(socket),
         true <- user_id == socket.assigns.current_user_id do
      SocketToken.expire_in(ms_left)
      {:ok, socket}
    else
      {:error, :token_expired} -> {:error, %{reason: "token_expired"}}
      _ -> {:error, %{reason: "not_found"}}
    end
  end

  @impl true
  def handle_in(_event, _payload, socket) do
    {:reply, {:error, %{type: "unknown-event"}}, socket}
  end

  @impl true
  def handle_info(:token_expired, socket) do
    push(socket, "token.expired", %{})
    {:stop, {:shutdown, :token_expired}, socket}
  end
end
