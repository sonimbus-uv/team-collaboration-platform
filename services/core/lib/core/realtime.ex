defmodule Core.Realtime do
  import Ecto.Query

  alias Core.Community.Channel
  alias Core.Repo
  alias CoreWeb.Endpoint

  def broadcast_to_user(user_id, event, payload) do
    Endpoint.broadcast("user:#{user_id}", event, envelope(event, payload))
  end

  def broadcast_to_channel(channel_id, event, payload) do
    Endpoint.broadcast("channel:#{channel_id}", event, envelope(event, payload))
  end

  def broadcast_to_group(group_id, event, payload) do
    message = envelope(event, payload)

    for channel_id <- channel_ids(group_id, :active) do
      Endpoint.broadcast("channel:#{channel_id}", event, message)
    end

    :ok
  end

  def disconnect_user_from_group(group_id, user_id) do
    for channel_id <- channel_ids(group_id, :all),
        topic <- ["channel:#{channel_id}", "voice:#{channel_id}"] do
      Endpoint.broadcast(topic, "access.revoked", %{user_id: user_id})
    end

    :ok
  end

  defp channel_ids(group_id, :active) do
    Repo.all(
      from c in Channel, where: c.group_id == ^group_id and is_nil(c.deleted_at), select: c.id
    )
  end

  defp channel_ids(group_id, :all) do
    Repo.all(from c in Channel, where: c.group_id == ^group_id, select: c.id)
  end

  defp envelope(event, payload), do: %{type: event, version: 1, payload: payload}
end
