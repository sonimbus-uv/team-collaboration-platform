defmodule Core.Fixtures do
  @moduledoc "Datos de prueba del contexto Community."

  alias Core.Community

  def user_id, do: Ecto.UUID.generate()

  def group_fixture(owner_id, attrs \\ %{}) do
    {:ok, group} = Community.create_group(owner_id, Enum.into(attrs, %{"name" => "Equipo expo"}))
    group
  end

  def invitation_fixture(group, creator_id, attrs \\ %{}) do
    {:ok, invitation} = Community.create_invitation(group.id, creator_id, attrs)
    invitation
  end

  def channel_fixture(group, admin_id, attrs \\ %{}) do
    attrs = Enum.into(attrs, %{"name" => "general", "type" => "text"})
    {:ok, channel} = Community.create_channel(group.id, admin_id, attrs)
    channel
  end

  def member_fixture(group, user_id, role) do
    Core.Repo.insert!(%Community.Member{group_id: group.id, user_id: user_id, role: role})
  end
end
