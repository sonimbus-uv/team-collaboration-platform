defmodule CoreWeb.MessageControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures

  alias Core.Messaging

  @moduletag :scylla

  setup_all do
    Core.ScyllaHelpers.await_scylla!()
  end

  setup %{conn: conn} do
    owner = user_id()
    group = group_fixture(owner)
    channel = channel_fixture(group, owner)
    guest = user_id()
    member_fixture(group, guest, :member)

    {:ok, message, :created} =
      Messaging.create_message(channel, group, owner, %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "hola"
      })

    %{
      owner: owner,
      group: group,
      channel: channel,
      message: message,
      conn: authenticate(conn, owner),
      guest_conn: authenticate(build_conn(), guest)
    }
  end

  defp path(ctx), do: ~p"/api/v1/channels/#{ctx.channel.id}/messages/#{ctx.message.id}"

  test "PATCH edits the message and answers with it", ctx do
    conn = patch(ctx.conn, path(ctx), %{"content" => "hola a todos"})

    assert %{"data" => %{"content" => "hola a todos", "edited_at" => edited_at}} =
             json_response(conn, 200)

    assert edited_at
  end

  test "PATCH by another member is 403", ctx do
    conn = patch(ctx.guest_conn, path(ctx), %{"content" => "x"})
    assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
  end

  test "PATCH with empty content is 422 and with an unknown id is 404", ctx do
    conn = patch(ctx.conn, path(ctx), %{"content" => ""})
    assert %{"errors" => %{"content" => [_ | _]}} = json_response(conn, 422)

    conn =
      patch(ctx.conn, ~p"/api/v1/channels/#{ctx.channel.id}/messages/#{Ecto.UUID.generate()}", %{
        "content" => "x"
      })

    assert json_response(conn, 404)
  end

  test "PATCH on an archived group is 409", ctx do
    ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Core.Repo.update!()

    conn = patch(ctx.conn, path(ctx), %{"content" => "x"})
    assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
  end

  test "DELETE answers 204 and the history shows the notice", ctx do
    conn = delete(ctx.conn, path(ctx))
    assert response(conn, 204)

    conn = get(ctx.conn, ~p"/api/v1/channels/#{ctx.channel.id}/messages")

    assert %{
             "data" => [
               %{
                 "deleted" => true,
                 "content" => nil,
                 "file_id" => nil,
                 "deleted_reason" => "author"
               }
             ]
           } = json_response(conn, 200)
  end

  test "DELETE by another member is 403 and a second delete is 404", ctx do
    conn = delete(ctx.guest_conn, path(ctx))
    assert json_response(conn, 403)

    assert response(delete(ctx.conn, path(ctx)), 204)
    assert json_response(delete(ctx.conn, path(ctx)), 404)
  end

  test "a non member gets 404", ctx do
    conn = delete(authenticate(build_conn(), user_id()), path(ctx))
    assert json_response(conn, 404)
  end
end
