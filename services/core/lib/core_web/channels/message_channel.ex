defmodule CoreWeb.MessageChannel do
  @moduledoc """
  Canal de Phoenix por canal de texto: tema `channel:<channel_id>` (CU-16).

  - Unirse exige ser miembro del grupo con un rol que alcance `min_role`
    (D-17) y un token sin vencer.
  - `message.create` (`client_message_id`, `content`, `reply_to` opcional)
    guarda el mensaje con idempotencia y responde con el mensaje. Solo el
    primer envío difunde `message.created` (versión 1, con el sobre de
    `contracts/events/README.md`) a los suscriptores;
    un reintento recibe la misma respuesta sin difusión.
  - El acceso se vuelve a comprobar en cada envío, por si el usuario dejó el
    grupo o fue expulsado después de unirse.
  - Al vencer el `exp` del token, el servidor envía `token.expired` y cierra
    el canal.

  Contrato en `contracts/events/message.created.md`.
  """
  use CoreWeb, :channel

  alias Core.{Community, Messaging}
  alias CoreWeb.{MessageJSON, Problem}

  @impl true
  def join("channel:" <> channel_id, _payload, socket) do
    user_id = socket.assigns.current_user_id

    with {:ok, ms_left} <- token_time_left(socket),
         {:ok, %{type: :text} = channel, _group, _role} <-
           Community.get_channel_for_member(channel_id, user_id) do
      Process.send_after(self(), :token_expired, ms_left)
      {:ok, assign(socket, :channel_id, channel.id)}
    else
      {:error, :token_expired} -> {:error, %{reason: "token_expired"}}
      _ -> {:error, %{reason: "not_found"}}
    end
  end

  @impl true
  def handle_in("message.create", payload, socket) when is_map(payload) do
    user_id = socket.assigns.current_user_id

    with {:ok, _ms_left} <- token_time_left(socket),
         {:ok, channel, group, _role} <-
           Community.get_channel_for_member(socket.assigns.channel_id, user_id),
         {:ok, message, outcome} <- Messaging.create_message(channel, group, user_id, payload) do
      data = MessageJSON.message(message)

      if outcome == :created do
        broadcast!(socket, "message.created", %{
          type: "message.created",
          version: 1,
          payload: data
        })
      end

      {:reply, {:ok, %{message: data}}, socket}
    else
      {:error, reason} -> {:reply, {:error, error_payload(reason)}, socket}
    end
  end

  def handle_in(_event, _payload, socket) do
    {:reply, {:error, %{type: "unknown-event"}}, socket}
  end

  @impl true
  def handle_info(:token_expired, socket) do
    push(socket, "token.expired", %{})
    {:stop, {:shutdown, :token_expired}, socket}
  end

  defp token_time_left(socket) do
    ms_left = socket.assigns.token_expires_at * 1_000 - System.system_time(:millisecond)
    if ms_left > 0, do: {:ok, ms_left}, else: {:error, :token_expired}
  end

  defp error_payload(%Ecto.Changeset{} = changeset),
    do: %{type: "validation-error", errors: Problem.changeset_errors(changeset)}

  defp error_payload(:token_expired), do: %{type: "token-expired"}
  defp error_payload(:not_found), do: %{type: "not-found"}
  defp error_payload(:group_archived), do: %{type: "group-archived"}
  defp error_payload(:client_message_id_conflict), do: %{type: "client-message-id-conflict"}
  defp error_payload(:unavailable), do: %{type: "messages-unavailable"}
  defp error_payload(_reason), do: %{type: "error"}
end
