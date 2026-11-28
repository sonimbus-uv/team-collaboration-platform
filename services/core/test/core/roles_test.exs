defmodule Core.RolesTest do
  use Core.DataCase, async: true

  import Core.Fixtures

  alias Core.Community
  alias CoreWeb.Endpoint

  setup do
    owner = user_id()
    group = group_fixture(owner)
    member = user_id()
    member_fixture(group, member, :member)
    %{owner: owner, group: group, member: member}
  end

  defp role_of(group, user) do
    {:ok, _group, role} = Community.get_group_for_member(group.id, user)
    role
  end

  defp permissions_of(group, role) do
    {:ok, roles} = Community.list_roles(group.id, group.created_by)
    Enum.find(roles, &(&1.role == role)).permissions
  end

  describe "change_role/4" do
    test "an admin promotes a member and the new role applies at once", ctx do
      assert {:error, :forbidden} =
               Community.create_invitation(ctx.group.id, ctx.member, %{})

      Endpoint.subscribe("user:#{ctx.member}")

      assert {:ok, %{user_id: user_id, role: :moderator}} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")

      assert user_id == ctx.member
      assert role_of(ctx.group, ctx.member) == :moderator
      assert {:ok, _invitation} = Community.create_invitation(ctx.group.id, ctx.member, %{})

      assert_receive %Phoenix.Socket.Broadcast{
        event: "member.role.changed",
        payload: %{
          type: "member.role.changed",
          version: 1,
          payload: %{role: :moderator, previous_role: :member}
        }
      }
    end

    test "demoting a moderator removes what the role allowed", ctx do
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "member")

      assert {:error, :forbidden} = Community.create_invitation(ctx.group.id, ctx.member, %{})
    end

    test "the same role is a no-op without an event", ctx do
      Endpoint.subscribe("user:#{ctx.member}")

      assert {:ok, %{role: :member}} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.member, "member")

      refute_receive %Phoenix.Socket.Broadcast{event: "member.role.changed"}
    end

    test "only admins change roles, even when a moderator is given other permissions", ctx do
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")

      assert {:error, :forbidden} =
               Community.change_role(ctx.group.id, ctx.member, ctx.owner, "member")

      assert {:error, :forbidden} =
               Community.change_role(ctx.group.id, ctx.member, ctx.member, "admin")
    end

    test "a non member actor or target gets not_found", ctx do
      assert {:error, :not_found} =
               Community.change_role(ctx.group.id, user_id(), ctx.member, "moderator")

      assert {:error, :not_found} =
               Community.change_role(ctx.group.id, ctx.owner, user_id(), "moderator")

      assert {:error, :not_found} =
               Community.change_role(ctx.group.id, ctx.owner, "not-a-uuid", "moderator")
    end

    test "an invalid role is a validation error", ctx do
      assert {:error, %Ecto.Changeset{} = changeset} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.member, "owner")

      assert %{role: ["is invalid"]} = errors_on(changeset)

      assert {:error, %Ecto.Changeset{}} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.member, nil)
    end

    test "the only admin cannot demote themselves", ctx do
      assert {:error, :last_admin} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.owner, "member")

      assert role_of(ctx.group, ctx.owner) == :admin
    end

    test "an admin can step down once another admin exists", ctx do
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "admin")

      assert {:ok, %{role: :member}} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.owner, "member")

      assert {:error, :last_admin} =
               Community.change_role(ctx.group.id, ctx.member, ctx.member, "moderator")
    end

    test "an archived group rejects role changes", ctx do
      ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Repo.update!()

      assert {:error, :group_archived} =
               Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")
    end
  end

  describe "list_roles/2" do
    test "any member sees the three roles with every permission", ctx do
      assert {:ok, [member, moderator, admin]} = Community.list_roles(ctx.group.id, ctx.member)
      assert [:member, :moderator, :admin] == [member.role, moderator.role, admin.role]
      assert map_size(admin.permissions) == length(Core.Authorization.permissions())
      assert moderator.permissions.moderate
      refute member.permissions.moderate
    end

    test "a non member gets not_found", ctx do
      assert {:error, :not_found} = Community.list_roles(ctx.group.id, user_id())
    end
  end

  describe "update_role_permissions/4" do
    test "turning a permission off applies to everyone with that role", ctx do
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")
      assert {:ok, _} = Community.create_invitation(ctx.group.id, ctx.member, %{})

      assert {:ok, %{role: :moderator, permissions: %{invite_members: false, moderate: true}}} =
               Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", %{
                 "invite_members" => false
               })

      assert {:error, :forbidden} = Community.create_invitation(ctx.group.id, ctx.member, %{})
    end

    test "turning a permission on grants it", ctx do
      {:ok, _} = Community.change_role(ctx.group.id, ctx.owner, ctx.member, "moderator")

      assert {:error, :forbidden} =
               Community.create_channel(ctx.group.id, ctx.member, %{
                 "name" => "nuevo",
                 "type" => "text"
               })

      {:ok, _} =
        Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", %{
          "manage_channels" => true
        })

      assert {:ok, _channel} =
               Community.create_channel(ctx.group.id, ctx.member, %{
                 "name" => "nuevo",
                 "type" => "text"
               })

      assert permissions_of(ctx.group, :moderator).manage_channels
    end

    test "unspecified permissions keep their value", ctx do
      {:ok, _} =
        Community.update_role_permissions(ctx.group.id, ctx.owner, "member", %{"moderate" => true})

      permissions = permissions_of(ctx.group, :member)
      assert permissions.moderate
      refute permissions.invite_members
    end

    test "admin-only, unknown and non boolean permissions are rejected", ctx do
      for changes <- [
            %{"manage_roles" => true},
            %{"manage_group" => false},
            %{"fly" => true},
            %{"moderate" => "yes"},
            %{}
          ] do
        assert {:error, %Ecto.Changeset{}} =
                 Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", changes)
      end

      assert {:error, %Ecto.Changeset{}} =
               Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", nil)

      refute permissions_of(ctx.group, :moderator).manage_roles
    end

    test "an invalid role is a validation error", ctx do
      assert {:error, %Ecto.Changeset{}} =
               Community.update_role_permissions(ctx.group.id, ctx.owner, "owner", %{
                 "moderate" => true
               })
    end

    test "non admins are forbidden and an archived group rejects the change", ctx do
      assert {:error, :forbidden} =
               Community.update_role_permissions(ctx.group.id, ctx.member, "member", %{
                 "moderate" => true
               })

      ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Repo.update!()

      assert {:error, :group_archived} =
               Community.update_role_permissions(ctx.group.id, ctx.owner, "member", %{
                 "moderate" => true
               })
    end
  end
end
