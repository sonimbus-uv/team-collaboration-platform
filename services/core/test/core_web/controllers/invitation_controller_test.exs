defmodule CoreWeb.InvitationControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures
  import Ecto.Query

  alias Core.Community
  alias Core.Community.Invitation
  alias Core.Repo

  setup %{conn: conn} do
    owner = user_id()
    group = group_fixture(owner)
    moderator = user_id()
    member = user_id()
    member_fixture(group, moderator, :moderator)
    member_fixture(group, member, :member)

    %{
      owner: owner,
      group: group,
      admin_conn: authenticate(conn, owner),
      moderator_conn: authenticate(build_conn(), moderator),
      member_conn: authenticate(build_conn(), member),
      stranger_conn: authenticate(build_conn(), user_id())
    }
  end

  describe "GET /groups/:id/invitations" do
    test "lists only usable invitations with remaining uses", ctx do
      open = invitation_fixture(ctx.group, ctx.owner)
      limited = invitation_fixture(ctx.group, ctx.owner, %{"max_uses" => 5})
      revoked = invitation_fixture(ctx.group, ctx.owner)
      expired = invitation_fixture(ctx.group, ctx.owner)
      exhausted = invitation_fixture(ctx.group, ctx.owner, %{"max_uses" => 1})

      Repo.update_all(from(i in Invitation, where: i.id == ^revoked.id),
        set: [revoked_at: DateTime.utc_now()]
      )

      Repo.update_all(from(i in Invitation, where: i.id == ^expired.id),
        set: [
          created_at: DateTime.add(DateTime.utc_now(), -7_200, :second),
          expires_at: DateTime.add(DateTime.utc_now(), -3_600, :second)
        ]
      )

      Repo.update_all(from(i in Invitation, where: i.id == ^exhausted.id),
        set: [uses: 1]
      )

      Repo.update_all(from(i in Invitation, where: i.id == ^limited.id),
        set: [uses: 2]
      )

      conn = get(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/invitations")
      %{"data" => data} = json_response(conn, 200)

      assert data |> Enum.map(& &1["id"]) |> Enum.sort() == Enum.sort([open.id, limited.id])
      assert Enum.find(data, &(&1["id"] == open.id))["remaining_uses"] == nil
      assert Enum.find(data, &(&1["id"] == limited.id))["remaining_uses"] == 3
      assert Enum.all?(data, &is_binary(&1["expires_at"]))
    end

    test "a moderator can list, a member gets 403, a stranger 404", ctx do
      path = ~p"/api/v1/groups/#{ctx.group.id}/invitations"

      assert %{"data" => []} = json_response(get(ctx.moderator_conn, path), 200)
      assert %{"type" => "/problems/forbidden"} = json_response(get(ctx.member_conn, path), 403)
      assert %{"type" => "/problems/not-found"} = json_response(get(ctx.stranger_conn, path), 404)
    end
  end

  describe "DELETE /groups/:id/invitations/:id" do
    test "revoking stops new joins and removes it from the list", ctx do
      invitation = invitation_fixture(ctx.group, ctx.owner)
      path = ~p"/api/v1/groups/#{ctx.group.id}/invitations/#{invitation.id}"

      assert response(delete(ctx.admin_conn, path), 204)

      assert %{"data" => []} =
               json_response(
                 get(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/invitations"),
                 200
               )

      conn =
        post(authenticate(build_conn(), user_id()), ~p"/api/v1/invitations/join", %{
          "code" => invitation.code
        })

      assert %{"type" => "/problems/invitation-invalid"} = json_response(conn, 422)
    end

    test "revoking twice is idempotent and keeps the first timestamp", ctx do
      invitation = invitation_fixture(ctx.group, ctx.owner)
      path = ~p"/api/v1/groups/#{ctx.group.id}/invitations/#{invitation.id}"

      assert response(delete(ctx.admin_conn, path), 204)
      first = Repo.get!(Invitation, invitation.id).revoked_at
      assert response(delete(ctx.moderator_conn, path), 204)
      assert Repo.get!(Invitation, invitation.id).revoked_at == first
    end

    test "members who already joined keep their membership", ctx do
      invitation = invitation_fixture(ctx.group, ctx.owner)
      guest = user_id()
      assert {:ok, :joined, _group, :member} = Community.join_group(invitation.code, guest)

      delete(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/invitations/#{invitation.id}")

      assert {:ok, _group, :member} = Community.get_group_for_member(ctx.group.id, guest)
    end

    test "a member gets 403 and the invitation stays usable", ctx do
      invitation = invitation_fixture(ctx.group, ctx.owner)
      path = ~p"/api/v1/groups/#{ctx.group.id}/invitations/#{invitation.id}"

      assert %{"type" => "/problems/forbidden"} =
               json_response(delete(ctx.member_conn, path), 403)

      assert Repo.get!(Invitation, invitation.id).revoked_at == nil
    end

    test "an invitation of another group or an unknown id is 404", ctx do
      other_owner = user_id()
      other = invitation_fixture(group_fixture(other_owner), other_owner)
      base = ~p"/api/v1/groups/#{ctx.group.id}/invitations"

      assert %{"type" => "/problems/not-found"} =
               json_response(delete(ctx.admin_conn, "#{base}/#{other.id}"), 404)

      assert %{"type" => "/problems/not-found"} =
               json_response(delete(ctx.admin_conn, "#{base}/#{Ecto.UUID.generate()}"), 404)

      assert %{"type" => "/problems/not-found"} =
               json_response(delete(ctx.admin_conn, "#{base}/no-es-uuid"), 404)

      assert Repo.get!(Invitation, other.id).revoked_at == nil
    end
  end

  describe "POST /groups/:id/invitations" do
    test "invalid parameters are a validation error", ctx do
      conn =
        post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/invitations", %{
          "expires_in" => 60,
          "max_uses" => 0
        })

      assert %{"type" => "/problems/validation-error"} = json_response(conn, 422)
    end
  end
end
