defmodule CoreWeb.ChannelControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers

  import Phoenix.ChannelTest,
    only: [subscribe_and_join: 3, assert_push: 2, refute_push: 2]

  alias Core.Community
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
      member_conn: authenticate(build_conn(), member)
    }
  end

  defp channel_path(ctx, channel \\ nil),
    do: ~p"/api/v1/groups/#{ctx.group.id}/channels/#{(channel || ctx.channel).id}"

  defp join_user_topic(user) do
    {:ok, socket} =
      Phoenix.ChannelTest.connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})

    {:ok, _, socket} = subscribe_and_join(socket, "user:#{user}", %{})
    socket
  end

  describe "PATCH" do
    test "the admin renames and describes the channel and members see it", ctx do
      join_user_topic(ctx.member)

      conn =
        patch(ctx.admin_conn, channel_path(ctx), %{
          "name" => "anuncios",
          "description" => "Avisos"
        })

      assert %{"data" => %{"name" => "anuncios", "description" => "Avisos"}} =
               json_response(conn, 200)

      channel_id = ctx.channel.id

      assert_push "channel.updated", %{
        type: "channel.updated",
        version: 1,
        payload: %{id: ^channel_id, name: "anuncios", description: "Avisos"}
      }

      conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/channels")
      assert %{"data" => [%{"name" => "anuncios"}]} = json_response(conn, 200)
    end

    test "does not notify members who cannot see the channel", ctx do
      restricted =
        channel_fixture(ctx.group, ctx.owner, %{"name" => "mods", "min_role" => "moderator"})

      join_user_topic(ctx.member)

      patch(ctx.admin_conn, channel_path(ctx, restricted), %{"name" => "moderadores"})

      refute_push "channel.updated", _
    end

    test "an unchanged payload does not emit an event", ctx do
      join_user_topic(ctx.member)

      conn = patch(ctx.admin_conn, channel_path(ctx), %{"name" => ctx.channel.name})

      assert json_response(conn, 200)
      refute_push "channel.updated", _
    end

    test "a repeated name is a conflict and an invalid name is a validation error", ctx do
      channel_fixture(ctx.group, ctx.owner, %{"name" => "random"})

      conn = patch(ctx.admin_conn, channel_path(ctx), %{"name" => "random"})
      assert %{"type" => "/problems/conflict"} = json_response(conn, 409)

      conn = patch(ctx.admin_conn, channel_path(ctx), %{"name" => "Nombre Inválido"})
      assert %{"type" => "/problems/validation-error"} = json_response(conn, 422)

      conn = patch(ctx.admin_conn, channel_path(ctx), %{"name" => ""})
      assert %{"type" => "/problems/validation-error"} = json_response(conn, 422)
    end

    test "type and min_role cannot be changed", ctx do
      conn =
        patch(ctx.admin_conn, channel_path(ctx), %{
          "name" => "otro",
          "type" => "voice",
          "min_role" => "admin"
        })

      assert %{"data" => %{"type" => "text", "min_role" => "member"}} = json_response(conn, 200)
    end

    test "a member and a moderator get 403, a stranger 404", ctx do
      conn = patch(ctx.member_conn, channel_path(ctx), %{"name" => "x"})
      assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)

      conn = patch(authenticate(build_conn(), ctx.moderator), channel_path(ctx), %{"name" => "x"})
      assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)

      conn = patch(authenticate(build_conn(), user_id()), channel_path(ctx), %{"name" => "x"})
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    test "a channel of another group or an unknown id is 404", ctx do
      other_owner = user_id()
      other = channel_fixture(group_fixture(other_owner), other_owner)

      conn = patch(ctx.admin_conn, channel_path(ctx, other), %{"name" => "x"})
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)

      conn = patch(ctx.admin_conn, channel_path(ctx, %{id: "no-es-uuid"}), %{"name" => "x"})
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    test "an archived group rejects the change", ctx do
      archive_group(ctx.group)

      conn = patch(ctx.admin_conn, channel_path(ctx), %{"name" => "x"})
      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
    end
  end

  describe "DELETE" do
    test "hides the channel, frees the name and stops serving its history", ctx do
      conn = delete(ctx.admin_conn, channel_path(ctx))
      assert response(conn, 204)

      conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/channels")
      assert %{"data" => []} = json_response(conn, 200)

      conn = get(ctx.member_conn, ~p"/api/v1/channels/#{ctx.channel.id}/messages")
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)

      assert {:error, :not_found} = Community.get_channel_for_member(ctx.channel.id, ctx.member)
      assert %{name: "general"} = channel_fixture(ctx.group, ctx.owner)
    end

    test "connected clients receive channel.deleted and the channel topic closes", ctx do
      Process.flag(:trap_exit, true)
      join_user_topic(ctx.member)

      {:ok, socket} =
        Phoenix.ChannelTest.connect(UserSocket, %{},
          connect_info: %{auth_token: token_for(ctx.member)}
        )

      {:ok, _, joined} = subscribe_and_join(socket, "channel:#{ctx.channel.id}", %{})
      ref = Process.monitor(joined.channel_pid)

      conn = delete(ctx.admin_conn, channel_path(ctx))
      assert response(conn, 204)

      channel_id = ctx.channel.id
      group_id = ctx.group.id

      assert_push "channel.deleted", %{
        type: "channel.deleted",
        version: 1,
        payload: %{id: ^channel_id, group_id: ^group_id}
      }

      assert_receive {:DOWN, ^ref, :process, _, {:shutdown, :channel_deleted}}
    end

    test "closes the voice topic", ctx do
      Process.flag(:trap_exit, true)
      voice = channel_fixture(ctx.group, ctx.owner, %{"name" => "sala", "type" => "voice"})

      {:ok, socket} =
        Phoenix.ChannelTest.connect(UserSocket, %{},
          connect_info: %{auth_token: token_for(ctx.member)}
        )

      {:ok, _, joined} = subscribe_and_join(socket, "voice:#{voice.id}", %{})
      ref = Process.monitor(joined.channel_pid)

      conn = delete(ctx.admin_conn, channel_path(ctx, voice))
      assert response(conn, 204)

      assert_receive {:DOWN, ^ref, :process, _, {:shutdown, :channel_deleted}}
    end

    test "deleting twice is 404", ctx do
      assert response(delete(ctx.admin_conn, channel_path(ctx)), 204)
      conn = delete(ctx.admin_conn, channel_path(ctx))
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    test "a member gets 403 and the channel stays", ctx do
      conn = delete(ctx.member_conn, channel_path(ctx))
      assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
      assert {:ok, _, _, _} = Community.get_channel_for_member(ctx.channel.id, ctx.member)
    end

    test "an archived group rejects the deletion", ctx do
      archive_group(ctx.group)

      conn = delete(ctx.admin_conn, channel_path(ctx))
      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
    end
  end

  defp archive_group(group) do
    import Ecto.Query

    Core.Repo.update_all(from(g in Core.Community.Group, where: g.id == ^group.id),
      set: [archived_at: DateTime.utc_now()]
    )
  end
end
