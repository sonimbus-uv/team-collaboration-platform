defmodule CoreWeb.VoiceChannel do
  use CoreWeb, :channel

  alias Core.{Community, Profiles}
  alias CoreWeb.{Presence, SocketToken}

  intercept ["access.revoked"]

  @impl true
  def join("voice:" <> channel_id, payload, socket) do
    user_id = socket.assigns.current_user_id

    with {:ok, ms_left} <- SocketToken.time_left(socket),
         {:ok, %{type: :voice} = channel, group, _role} <-
           Community.get_channel_for_member(channel_id, user_id),
         :ok <- require_active(group),
         {:ok, state} <- initial_state(payload) do
      SocketToken.expire_in(ms_left)
      send(self(), :after_join)
      {:ok, socket |> assign(:channel_id, channel.id) |> assign(:voice_state, state)}
    else
      {:error, :token_expired} -> {:error, %{reason: "token_expired"}}
      {:error, :group_archived} -> {:error, %{reason: "group_archived"}}
      {:error, :invalid_payload} -> {:error, %{reason: "invalid_payload"}}
      _ -> {:error, %{reason: "not_found"}}
    end
  end

  @impl true
  def handle_in("voice.update", payload, socket) when is_map(payload) do
    case state_changes(payload) do
      {:ok, changes} when map_size(changes) > 0 ->
        user_id = socket.assigns.current_user_id
        {:ok, _} = Presence.update(socket, user_id, &Map.merge(&1, changes))
        {:reply, {:ok, %{}}, socket}

      {:ok, _empty} ->
        {:reply,
         {:error, %{type: "validation-error", errors: %{mic: ["mic or camera is required"]}}},
         socket}

      {:error, errors} ->
        {:reply, {:error, %{type: "validation-error", errors: errors}}, socket}
    end
  end

  def handle_in(_event, _payload, socket) do
    {:reply, {:error, %{type: "unknown-event"}}, socket}
  end

  @impl true
  def handle_out("access.revoked", %{user_id: user_id}, socket) do
    if user_id == socket.assigns.current_user_id do
      {:stop, {:shutdown, :access_revoked}, socket}
    else
      {:noreply, socket}
    end
  end

  @impl true
  def handle_info(:after_join, socket) do
    user_id = socket.assigns.current_user_id
    %{^user_id => display_name} = Profiles.display_names([user_id])

    meta =
      Map.merge(socket.assigns.voice_state, %{
        display_name: display_name,
        joined_at: DateTime.to_iso8601(DateTime.utc_now())
      })

    {:ok, _} = Presence.track(socket, user_id, meta)
    push(socket, "presence_state", Presence.list(socket))
    {:noreply, socket}
  end

  def handle_info(:token_expired, socket) do
    push(socket, "token.expired", %{})
    {:stop, {:shutdown, :token_expired}, socket}
  end

  defp require_active(%{archived_at: nil}), do: :ok
  defp require_active(_group), do: {:error, :group_archived}

  defp initial_state(payload) when is_map(payload) do
    case state_changes(payload) do
      {:ok, changes} -> {:ok, Map.merge(%{mic: false, camera: false}, changes)}
      {:error, _errors} -> {:error, :invalid_payload}
    end
  end

  defp initial_state(_payload), do: {:error, :invalid_payload}

  defp state_changes(payload) do
    {changes, errors} =
      Enum.reduce([mic: "mic", camera: "camera"], {%{}, %{}}, fn {field, key},
                                                                 {changes, errors} ->
        case Map.fetch(payload, key) do
          :error -> {changes, errors}
          {:ok, value} when is_boolean(value) -> {Map.put(changes, field, value), errors}
          {:ok, _other} -> {changes, Map.put(errors, field, ["must be a boolean"])}
        end
      end)

    if map_size(errors) == 0, do: {:ok, changes}, else: {:error, errors}
  end
end
