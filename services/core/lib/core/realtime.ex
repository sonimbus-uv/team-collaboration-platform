defmodule Core.Realtime do
  import Ecto.Query

  alias Core.Community.{Channel, Member}
  alias Core.Repo
  alias CoreWeb.Endpoint

  def broadcast_to_user(user_id, event, payload) do
    Endpoint.broadcast("user:#{user_id}", event, envelope(event, payload))
  end

  def broadcast_to_group(group_id, event, payload) do
    message = envelope(event, payload)

    for channel_id <- channel_ids(group_id, :active) do
      Endpoint.broadcast("channel:#{channel_id}", event, message)
    end

    :ok
  end

  def broadcast_to_members(group_id, event, payload, min_role \\ :member) do
    message = envelope(event, payload)

    from(m in Member, where: m.group_id == ^group_id, select: {m.user_id, m.role})
    |> Repo.all()
    |> Enum.filter(fn {_user_id, role} -> Member.role_at_least?(role, min_role) end)
    |> Enum.each(fn {user_id, _role} ->
      Endpoint.broadcast("user:#{user_id}", event, message)
    end)

    :ok
  end

  def close_channel(%Channel{id: channel_id, type: type}) do
    if type == :voice do
      for user_id <- CoreWeb.Presence.list("voice:#{channel_id}") |> Map.keys() do
        Task.Supervisor.start_child(Core.TaskSupervisor, fn ->
          Core.Voice.remove_participant(channel_id, user_id)
        end)
      end
    end

    for topic <- ["channel:#{channel_id}", "voice:#{channel_id}"] do
      Endpoint.broadcast(topic, "channel.closed", %{})
    end

    :ok
  end

  def close_group(group_id) do
    Repo.all(from c in Channel, where: c.group_id == ^group_id and is_nil(c.deleted_at))
    |> Enum.each(&close_channel/1)
  end

  def disconnect_user_from_group(group_id, user_id) do
    for channel_id <- channel_ids(group_id, :all),
        topic <- ["channel:#{channel_id}", "voice:#{channel_id}"] do
      Endpoint.broadcast(topic, "access.revoked", %{user_id: user_id})
    end

    remove_from_voice_rooms(group_id, user_id)

    :ok
  end

  defp remove_from_voice_rooms(group_id, user_id) do
    voice_ids =
      Repo.all(
        from c in Channel, where: c.group_id == ^group_id and c.type == :voice, select: c.id
      )

    for channel_id <- voice_ids do
      Task.Supervisor.start_child(Core.TaskSupervisor, fn ->
        Core.Voice.remove_participant(channel_id, user_id)
      end)
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
