defmodule Core.Authorization do
  import Ecto.Query

  alias Core.Authorization.{Cache, RolePermission}
  alias Core.Community.Member
  alias Core.Repo

  @permissions RolePermission.permissions()
  @admin_only [:manage_group, :manage_roles]
  @defaults %{
    member: [],
    moderator: [:invite_members, :moderate],
    admin: @permissions
  }

  def permissions, do: @permissions

  def admin_only, do: @admin_only

  def adjustable, do: @permissions -- @admin_only

  def defaults(role), do: Map.fetch!(@defaults, role)

  def seed(group_id) do
    rows =
      for role <- Member.roles(), permission <- @permissions do
        %{
          group_id: group_id,
          role: role,
          permission: permission,
          enabled: permission in defaults(role)
        }
      end

    {_count, _rows} = Repo.insert_all(RolePermission, rows)
    :ok
  end

  def can?(group_id, role, permission), do: permission in permissions(group_id, role)

  def authorize(group_id, role, permission) do
    if can?(group_id, role, permission), do: :ok, else: {:error, :forbidden}
  end

  def permissions(group_id, role) do
    key = {group_id, role}

    case Cache.fetch(key) do
      {:ok, value} -> value
      :miss -> Cache.put(key, load(group_id, role))
    end
  end

  def invalidate_group(group_id), do: Cache.delete_group(group_id)

  defp load(group_id, role) do
    rows =
      Repo.all(
        from r in RolePermission,
          where: r.group_id == ^group_id and r.role == ^role,
          select: {r.permission, r.enabled}
      )

    enabled =
      case rows do
        [] -> defaults(role)
        rows -> for {permission, true} <- rows, do: permission
      end

    clamp(role, enabled)
  end

  defp clamp(:admin, enabled), do: Enum.uniq(enabled ++ @admin_only)
  defp clamp(_role, enabled), do: enabled -- @admin_only
end
