defmodule CoreWeb.LeaveGroupControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers
  import Ecto.Query

  import Phoenix.ChannelTest,
    only: [subscribe_and_join: 3, assert_push: 2]

  alias Core.Community
  alias Core.Community.{Group, Member}
  alias Core.Repo
  alias CoreWeb.UserSocket

  @endpoint CoreWeb.Endpoint

  setup %{conn: conn} do
    owner = user_id()
    group = group_fixture(owner)
    member = user_id()
    moderator = user_id()
    member_fixture(group, member, :member)
    member_fixture(group, moderator, :moderator)
    channel = channel_fixture(group, owner)

    %{
      owner: owner,
      member: member,
      moderator: moderator,
      group: group,
      channel: channel,
      admin_conn: authenticate(conn, owner),
      member_conn: authenticate(build_conn(), member),
      moderator_conn: authenticate(build_conn(), moderator)
    }
  end

  defp leave_path(ctx), do: ~p"/api/v1/groups/#{ctx.group.id}/members/me"

  defp connect_as(user),
    do: Phoenix.ChannelTest.connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})

  defp join_user_topic(user) do
    {:ok, socket} = connect_as(user)
    {:ok, _, socket} = subscribe_and_join(socket, "user:#{user}", %{})
    socket
  end

  test "a member leaves and stops seeing the group", ctx do
    assert response(delete(ctx.member_conn, leave_path(ctx)), 204)

    conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}")
    assert %{"type" => "/problems/not-found"} = json_response(conn, 404)

    conn = get(ctx.member_conn, ~p"/api/v1/groups")
    assert %{"data" => []} = json_response(conn, 200)

    refute Repo.exists?(
             from m in Member, where: m.group_id == ^ctx.group.id and m.user_id == ^ctx.member
           )
  end

  test "the remaining members are told", ctx do
    join_user_topic(ctx.owner)

    assert response(delete(ctx.member_conn, leave_path(ctx)), 204)

    group_id = ctx.group.id
    member = ctx.member

    assert_push "member.left", %{
      type: "member.left",
      version: 1,
      payload: %{group_id: ^group_id, user_id: ^member}
    }
  end

  test "the leaver's channel topics close", ctx do
    Process.flag(:trap_exit, true)
    {:ok, socket} = connect_as(ctx.member)
    {:ok, _, joined} = subscribe_and_join(socket, "channel:#{ctx.channel.id}", %{})
    ref = Process.monitor(joined.channel_pid)

    assert response(delete(ctx.member_conn, leave_path(ctx)), 204)

    assert_receive {:DOWN, ^ref, :process, _, {:shutdown, :access_revoked}}
  end

  test "an admin can leave when another admin remains", ctx do
    member_fixture(ctx.group, user_id(), :admin)
    Community.change_role(ctx.group.id, ctx.owner, ctx.moderator, "admin")

    assert response(delete(ctx.admin_conn, leave_path(ctx)), 204)
    assert {:error, :not_found} = Community.get_group_for_member(ctx.group.id, ctx.owner)
  end

  test "the only admin gets a conflict and keeps the membership", ctx do
    conn = delete(ctx.admin_conn, leave_path(ctx))
    assert %{"type" => "/problems/last-admin"} = json_response(conn, 409)

    assert {:ok, _group, :admin} = Community.get_group_for_member(ctx.group.id, ctx.owner)
  end

  test "leaving is allowed in an archived group", ctx do
    Repo.update_all(from(g in Group, where: g.id == ^ctx.group.id),
      set: [archived_at: DateTime.utc_now()]
    )

    assert response(delete(ctx.member_conn, leave_path(ctx)), 204)
  end

  test "a stranger and a repeated leave are 404", ctx do
    conn = delete(authenticate(build_conn(), user_id()), leave_path(ctx))
    assert %{"type" => "/problems/not-found"} = json_response(conn, 404)

    assert response(delete(ctx.member_conn, leave_path(ctx)), 204)
    conn = delete(ctx.member_conn, leave_path(ctx))
    assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
  end

  test "someone who left can join again with an invitation as a member", ctx do
    invitation = invitation_fixture(ctx.group, ctx.owner)
    assert response(delete(ctx.moderator_conn, leave_path(ctx)), 204)

    conn = post(ctx.moderator_conn, ~p"/api/v1/invitations/join", %{"code" => invitation.code})
    assert %{"role" => "member"} = json_response(conn, 201)["data"]
  end
end
