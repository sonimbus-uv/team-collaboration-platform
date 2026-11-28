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

    {:ok, _} = Core.Profiles.put_display_name(ctx.owner, %{"display_name" => "Ana"})
    owner = ctx.owner

    ref = push(socket, "message.create", payload)

    assert_reply ref, :ok, %{
      message: %{id: id, content: "hola", author: %{id: ^owner, display_name: "Ana"}}
    }

    assert_broadcast "message.created", %{
      type: "message.created",
      version: 1,
      payload: %{id: ^id}
    }

    ref = push(socket, "message.create", payload)
    assert_reply ref, :ok, %{message: %{id: ^id}}
    refute_broadcast "message.created", _
  end

  test "envía un mensaje con archivo y lo difunde con su file_id", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})
    file_id = Ecto.UUID.generate()

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "",
        "file_id" => file_id
      })

    assert_reply ref, :ok, %{message: %{id: id, content: "", file_id: ^file_id}}

    assert_broadcast "message.created", %{
      type: "message.created",
      payload: %{id: ^id, file_id: ^file_id}
    }
  end

  test "un mensaje sin texto ni archivo es un error de validación", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => ""
      })

    assert_reply ref, :error, %{type: "validation-error", errors: %{content: [_]}}
  end

  test "edita y elimina un mensaje propio y difunde ambos cambios", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "hola"
      })

    assert_reply ref, :ok, %{message: %{id: id}}

    ref = push(socket, "message.update", %{"id" => id, "content" => "hola a todos"})
    assert_reply ref, :ok, %{message: %{id: ^id, content: "hola a todos", edited_at: %DateTime{}}}

    assert_push "message.updated", %{
      type: "message.updated",
      version: 1,
      payload: %{id: ^id, content: "hola a todos"}
    }

    ref = push(socket, "message.delete", %{"id" => id})
    assert_reply ref, :ok, %{}

    assert_push "message.deleted", %{
      type: "message.deleted",
      version: 1,
      payload: %{id: ^id, deleted_reason: "author"}
    }
  end

  test "otro miembro no puede editar ni eliminar mensajes ajenos", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "mio"
      })

    assert_reply ref, :ok, %{message: %{id: id}}

    guest = user_id()
    member_fixture(ctx.group, guest, :member)
    {:ok, guest_socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(guest)})
    {:ok, _, guest_socket} = subscribe_and_join(guest_socket, "channel:#{ctx.channel.id}", %{})

    ref = push(guest_socket, "message.update", %{"id" => id, "content" => "x"})
    assert_reply ref, :error, %{type: "forbidden"}

    ref = push(guest_socket, "message.delete", %{"id" => id})
    assert_reply ref, :error, %{type: "forbidden"}
  end

  test "un moderador elimina un mensaje ajeno y el canal ve el motivo", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    guest = user_id()
    member_fixture(ctx.group, guest, :member)
    {:ok, guest_socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(guest)})
    {:ok, _, guest_socket} = subscribe_and_join(guest_socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(guest_socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "ofensivo"
      })

    assert_reply ref, :ok, %{message: %{id: id}}

    ref = push(socket, "message.delete", %{"id" => id, "reason" => "Spam"})
    assert_reply ref, :ok, %{}

    assert_push "message.deleted", %{payload: %{id: ^id, deleted_reason: "moderation"}}
  end

  test "reacciona por el canal, alterna y difunde reaction.updated", ctx do
    {:ok, _, socket} = subscribe_and_join(ctx.socket, "channel:#{ctx.channel.id}", %{})

    ref =
      push(socket, "message.create", %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => "hola"
      })

    assert_reply ref, :ok, %{message: %{id: id}}
    owner = ctx.owner

    ref = push(socket, "reaction.toggle", %{"message_id" => id, "emoji" => "👍"})
    assert_reply ref, :ok, %{emoji: "👍", count: 1, active: true}

    assert_push "reaction.updated", %{
      type: "reaction.updated",
      version: 1,
      payload: %{message_id: ^id, emoji: "👍", count: 1, active: true, user_id: ^owner}
    }

    ref = push(socket, "reaction.toggle", %{"message_id" => id, "emoji" => "👍"})
    assert_reply ref, :ok, %{count: 0, active: false}

    ref = push(socket, "reaction.toggle", %{"message_id" => id, "emoji" => "a"})
    assert_reply ref, :error, %{type: "validation-error", errors: %{emoji: [_]}}
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

    owner = ctx.owner

    assert %{
             "data" => [
               %{"content" => "hola", "author" => %{"id" => ^owner, "display_name" => nil}}
             ],
             "next_cursor" => nil
           } =
             Phoenix.ConnTest.json_response(conn, 200)
  end
end
