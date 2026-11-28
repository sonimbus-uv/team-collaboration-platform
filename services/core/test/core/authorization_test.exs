defmodule Core.AuthorizationTest do
  use Core.DataCase, async: true

  import Core.Fixtures

  alias Core.Authorization
  alias Core.Authorization.RolePermission
  alias Core.Community.Group

  setup do
    owner = user_id()
    %{owner: owner, group: group_fixture(owner)}
  end

  defp set_permission(group, role, permission, enabled) do
    from(r in RolePermission,
      where: r.group_id == ^group.id and r.role == ^role and r.permission == ^permission
    )
    |> Repo.update_all(set: [enabled: enabled])
  end

  test "creating a group seeds every role and permission", %{group: group} do
    assert Repo.aggregate(from(r in RolePermission, where: r.group_id == ^group.id), :count) ==
             3 * length(Authorization.permissions())
  end

  test "members have no permissions by default", %{group: group} do
    for permission <- Authorization.permissions() do
      refute Authorization.can?(group.id, :member, permission)
    end
  end

  test "moderators can invite and moderate by default", %{group: group} do
    assert Authorization.can?(group.id, :moderator, :invite_members)
    assert Authorization.can?(group.id, :moderator, :moderate)
    refute Authorization.can?(group.id, :moderator, :manage_channels)
    refute Authorization.can?(group.id, :moderator, :manage_projects)
  end

  test "admins have every permission by default", %{group: group} do
    for permission <- Authorization.permissions() do
      assert Authorization.can?(group.id, :admin, permission)
    end
  end

  test "a group without rows falls back to the defaults" do
    group = Repo.insert!(%Group{name: "Sin filas", created_by: user_id()})

    assert Authorization.can?(group.id, :moderator, :moderate)
    refute Authorization.can?(group.id, :member, :moderate)
    assert Authorization.can?(group.id, :admin, :manage_roles)
  end

  test "admin keeps group and role management even if the rows say otherwise", %{group: group} do
    set_permission(group, :admin, :manage_roles, false)
    set_permission(group, :admin, :manage_group, false)
    Authorization.invalidate_group(group.id)

    assert Authorization.can?(group.id, :admin, :manage_roles)
    assert Authorization.can?(group.id, :admin, :manage_group)
  end

  test "other roles never get admin-only permissions", %{group: group} do
    set_permission(group, :moderator, :manage_roles, true)
    Authorization.invalidate_group(group.id)

    refute Authorization.can?(group.id, :moderator, :manage_roles)
  end

  test "the cache serves the stored value until the group is invalidated", %{group: group} do
    assert Authorization.can?(group.id, :moderator, :moderate)

    set_permission(group, :moderator, :moderate, false)
    assert Authorization.can?(group.id, :moderator, :moderate)

    Authorization.invalidate_group(group.id)
    refute Authorization.can?(group.id, :moderator, :moderate)
  end

  test "invalidating a group leaves other groups cached", %{group: group, owner: owner} do
    other = group_fixture(owner, %{"name" => "Otro"})
    assert Authorization.can?(other.id, :moderator, :moderate)
    set_permission(other, :moderator, :moderate, false)

    Authorization.invalidate_group(group.id)

    assert Authorization.can?(other.id, :moderator, :moderate)
  end

  test "authorize returns forbidden when the permission is missing", %{group: group} do
    assert :ok = Authorization.authorize(group.id, :admin, :manage_channels)
    assert {:error, :forbidden} = Authorization.authorize(group.id, :member, :manage_channels)
  end

  test "the database rejects permissions outside the catalog", %{group: group} do
    assert_raise Postgrex.Error, ~r/group_role_permissions_catalog/, fn ->
      Repo.insert_all("group_role_permissions", [
        %{group_id: Ecto.UUID.dump!(group.id), role: "member", permission: "fly", enabled: true}
      ])
    end
  end
end
