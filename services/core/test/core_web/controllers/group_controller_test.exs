defmodule CoreWeb.GroupControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers
  import Ecto.Query

  import Phoenix.ChannelTest,
    only: [subscribe_and_join: 3, assert_push: 2, refute_push: 2, assert_reply: 3]

  alias Core.Community.Group
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

  defp group_path(ctx), do: ~p"/api/v1/groups/#{ctx.group.id}"
  defp archive_path(ctx), do: ~p"/api/v1/groups/#{ctx.group.id}/archive"

  defp connect_as(user),
    do: Phoenix.ChannelTest.connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})

  defp join_user_topic(user) do
    {:ok, socket} = connect_as(user)
    {:ok, _, socket} = subscribe_and_join(socket, "user:#{user}", %{})
    socket
  end

  defp archive!(group) do
    Repo.update_all(from(g in Group, where: g.id == ^group.id),
      set: [archived_at: DateTime.utc_now()]
    )
  end

  describe "PATCH /groups/:id" do
    test "the admin edits the group and members see the change", ctx do
      join_user_topic(ctx.member)

      conn =
        patch(ctx.admin_conn, group_path(ctx), %{
          "name" => "  Equipo final ",
          "description" => "Todo el equipo"
        })

      assert %{"data" => %{"name" => "Equipo final", "description" => "Todo el equipo"}} =
               json_response(conn, 200)

      group_id = ctx.group.id

      assert_push "group.updated", %{
        type: "group.updated",
        version: 1,
        payload: %{id: ^group_id, name: "Equipo final", description: "Todo el equipo"}
      }

      conn = get(ctx.member_conn, group_path(ctx))

      assert %{"data" => %{"name" => "Equipo final", "role" => "member"}} =
               json_response(conn, 200)
    end

    test "an unchanged payload emits no event", ctx do
      join_user_topic(ctx.member)

      conn = patch(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name})

      assert json_response(conn, 200)
      refute_push "group.updated", _
    end

    test "invalid data is a validation error and changes nothing", ctx do
      for params <- [
            %{"name" => "x"},
            %{"name" => ""},
            %{"description" => String.duplicate("a", 501)}
          ] do
        conn = patch(ctx.admin_conn, group_path(ctx), params)
        assert %{"type" => "/problems/validation-error"} = json_response(conn, 422)
      end

      assert Repo.get!(Group, ctx.group.id).name == ctx.group.name
    end

    test "only the admin can edit; a stranger gets 404", ctx do
      for conn <- [ctx.member_conn, ctx.moderator_conn] do
        conn = patch(conn, group_path(ctx), %{"name" => "Otro"})
        assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
      end

      conn = patch(authenticate(build_conn(), user_id()), group_path(ctx), %{"name" => "Otro"})
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    test "an archived group rejects the edit", ctx do
      archive!(ctx.group)

      conn = patch(ctx.admin_conn, group_path(ctx), %{"name" => "Otro"})
      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
    end
  end

  describe "POST /groups/:id/archive" do
    test "archives the group, notifies members and keeps the membership", ctx do
      join_user_topic(ctx.member)

      conn = post(ctx.admin_conn, archive_path(ctx))
      assert %{"data" => %{"archived_at" => archived_at}} = json_response(conn, 200)
      assert is_binary(archived_at)

      group_id = ctx.group.id

      assert_push "group.archived", %{
        type: "group.archived",
        version: 1,
        payload: %{id: ^group_id, archived_at: %DateTime{}}
      }

      conn = get(ctx.member_conn, group_path(ctx))

      assert %{"data" => %{"archived_at" => ^archived_at, "role" => "member"}} =
               json_response(conn, 200)
    end

    test "leaves the group read only for writes", ctx do
      post(ctx.admin_conn, archive_path(ctx))

      conn = post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/invitations", %{})
      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)

      conn =
        post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/channels", %{
          "name" => "nuevo",
          "type" => "text"
        })

      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)

      conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/channels")
      assert %{"data" => [%{"name" => "general"}]} = json_response(conn, 200)
    end

    test "archiving twice is 409", ctx do
      assert json_response(post(ctx.admin_conn, archive_path(ctx)), 200)

      conn = post(ctx.admin_conn, archive_path(ctx))
      assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
    end

    test "only the admin can archive", ctx do
      conn = post(ctx.moderator_conn, archive_path(ctx))
      assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
      assert Repo.get!(Group, ctx.group.id).archived_at == nil

      conn = post(authenticate(build_conn(), user_id()), archive_path(ctx))
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    @tag :scylla
    test "messages are rejected over the socket and the history stays readable", ctx do
      Core.ScyllaHelpers.await_scylla!()
      {:ok, socket} = connect_as(ctx.owner)
      {:ok, _, socket} = subscribe_and_join(socket, "channel:#{ctx.channel.id}", %{})

      ref =
        Phoenix.ChannelTest.push(socket, "message.create", %{
          "client_message_id" => Ecto.UUID.generate(),
          "content" => "antes"
        })

      assert_reply ref, :ok, %{message: %{content: "antes"}}

      post(ctx.admin_conn, archive_path(ctx))

      ref =
        Phoenix.ChannelTest.push(socket, "message.create", %{
          "client_message_id" => Ecto.UUID.generate(),
          "content" => "después"
        })

      assert_reply ref, :error, %{type: "group-archived"}

      conn = get(ctx.member_conn, ~p"/api/v1/channels/#{ctx.channel.id}/messages")
      assert %{"data" => [%{"content" => "antes"}]} = json_response(conn, 200)
    end
  end

  describe "DELETE /groups/:id" do
    test "needs the exact name; a different one cancels", ctx do
      for body <- [%{"name" => "equipo expo"}, %{"name" => "Otro"}, %{}] do
        conn = delete(ctx.admin_conn, group_path(ctx), body)
        assert %{"type" => "/problems/validation-error"} = json_response(conn, 422)
      end

      assert Repo.get!(Group, ctx.group.id).deleted_at == nil
      assert json_response(get(ctx.member_conn, group_path(ctx)), 200)
    end

    test "deletes the group and revokes access for everyone", ctx do
      conn = delete(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name})
      assert response(conn, 204)

      for conn <- [ctx.admin_conn, ctx.member_conn, ctx.moderator_conn] do
        assert %{"type" => "/problems/not-found"} = json_response(get(conn, group_path(ctx)), 404)
      end

      conn = get(ctx.admin_conn, ~p"/api/v1/groups")
      assert %{"data" => []} = json_response(conn, 200)

      conn = get(ctx.member_conn, ~p"/api/v1/channels/#{ctx.channel.id}/messages")
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end

    test "notifies members and closes the channel topics", ctx do
      Process.flag(:trap_exit, true)
      join_user_topic(ctx.member)
      {:ok, socket} = connect_as(ctx.member)
      {:ok, _, joined} = subscribe_and_join(socket, "channel:#{ctx.channel.id}", %{})
      ref = Process.monitor(joined.channel_pid)

      conn = delete(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name})
      assert response(conn, 204)

      group_id = ctx.group.id
      assert_push "group.deleted", %{type: "group.deleted", version: 1, payload: %{id: ^group_id}}
      assert_receive {:DOWN, ^ref, :process, _, {:shutdown, :channel_deleted}}
    end

    test "an archived group can be deleted", ctx do
      archive!(ctx.group)

      conn = delete(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name})
      assert response(conn, 204)
    end

    test "only the admin can delete", ctx do
      conn = delete(ctx.moderator_conn, group_path(ctx), %{"name" => ctx.group.name})
      assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)

      conn =
        delete(authenticate(build_conn(), user_id()), group_path(ctx), %{"name" => ctx.group.name})

      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
      assert Repo.get!(Group, ctx.group.id).deleted_at == nil
    end

    test "deleting twice is 404", ctx do
      assert response(delete(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name}), 204)

      conn = delete(ctx.admin_conn, group_path(ctx), %{"name" => ctx.group.name})
      assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
    end
  end
end
