defmodule CoreWeb.MessageChannelTest do
  use CoreWeb.ChannelCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers

  alias CoreWeb.UserSocket

  require Phoenix.ConnTest

  @moduletag :scylla

  setup_all do
    Core.ScyllaHelpers.await_scylla!()
  end

  setup do
    owner = user_id()
    group = group_fixture(owner)
    channel = channel_fixture(group, owner)
    {:ok, socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(owner)})
    %{owner: owner, group: group, channel: channel, socket: socket}
  end

  test "rechaza la conexión sin token o con token inválido" do
    assert {:error, :unauthorized} = connect(UserSocket, %{}, connect_info: %{})

    assert {:error, :unauthorized} =
             connect(UserSocket, %{"token" => "x.y.z"}, connect_info: %{})
  end

  test "un no miembro no puede unirse", %{channel: channel} do
    {:ok, stranger} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user_id())})

    assert {:error, %{reason: "not_found"}} =
             subscribe_and_join(stranger, "channel:#{channel.id}", %{})
  end

  test "envía, difunde una vez y responde igual al reintento", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})
    payload = %{"client_message_id" => Ecto.UUID.generate(), "content" => "hola"}

    ref = push(socket, "message.create", payload)
    assert_reply ref, :ok, %{message: %{id: id, content: "hola"}}

    assert_broadcast "message.created", %{
      type: "message.created",
      version: 1,
      payload: %{id: ^id}
    }

    ref = push(socket, "message.create", payload)
    assert_reply ref, :ok, %{message: %{id: ^id}}
    refute_broadcast "message.created", _
  end

  test "errores de validación en la respuesta", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref = push(socket, "message.create", %{"client_message_id" => Ecto.UUID.generate()})
    assert_reply ref, :error, %{type: "validation-error", errors: %{content: [_]}}
  end

  test "historial por REST", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "hola"
      })

    assert_reply ref, :ok, _

    conn =
      Phoenix.ConnTest.build_conn()
      |> CoreWeb.ConnCase.authenticate(ctx.owner)
      |> Phoenix.ConnTest.get("/api/v1/channels/#{ctx.channel.id}/messages")

    assert %{"data" => [%{"content" => "hola"}], "next_cursor" => nil} =
             Phoenix.ConnTest.json_response(conn, 200)
  end
end
