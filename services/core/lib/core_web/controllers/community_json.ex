defmodule CoreWeb.CommunityJSON do
  @moduledoc "Representación JSON de grupos, invitaciones y canales."

  alias Core.Community.{Channel, Group, Invitation}

  def group(%Group{} = group, role) do
    %{
      id: group.id,
      name: group.name,
      description: group.description,
      image_id: group.image_id,
      created_by: group.created_by,
      role: role,
      archived_at: group.archived_at,
      created_at: group.inserted_at
    }
  end

  def invitation(%Invitation{} = invitation) do
    %{
      id: invitation.id,
      group_id: invitation.group_id,
      code: invitation.code,
      expires_at: invitation.expires_at,
      max_uses: invitation.max_uses,
      uses: invitation.uses,
      created_at: invitation.created_at
    }
  end

  def channel(%Channel{} = channel) do
    %{
      id: channel.id,
      group_id: channel.group_id,
      name: channel.name,
      description: channel.description,
      type: channel.type,
      min_role: channel.min_role,
      created_at: channel.inserted_at
    }
  end
end
