defmodule Core.MessagingTest do
  use Core.DataCase, async: true

  import Core.Fixtures

  alias Core.Messaging
  alias Core.Messaging.TimeUUID

  @moduletag :scylla

  setup_all do
    Core.ScyllaHelpers.await_scylla!()
  end

  setup do
    owner = user_id()
    group = group_fixture(owner)
    %{owner: owner, group: group, channel: channel_fixture(group, owner)}
  end

  defp send_message(ctx, content, extra \\ %{}) do
    attrs = Map.merge(%{"client_message_id" => Ecto.UUID.generate(), "content" => content}, extra)
    Messaging.create_message(ctx.channel, ctx.group, ctx.owner, attrs)
  end

  test "guarda un mensaje con id generado por el servidor", ctx do
    assert {:ok, message, :created} = send_message(ctx, "hola")
    assert message.content == "hola"
    assert message.author_id == ctx.owner
    assert {:ok, _} = TimeUUID.to_datetime(message.id)
  end

  test "un reintento con el mismo client_message_id no duplica", ctx do
    client_id = Ecto.UUID.generate()

    assert {:ok, first, :created} = send_message(ctx, "hola", %{"client_message_id" => client_id})

    assert {:ok, again, :duplicate} =
             send_message(ctx, "hola", %{"client_message_id" => client_id})

    assert again.id == first.id

    assert {:ok, %{messages: [only]}} = Messaging.list_messages(ctx.channel, %{})
    assert only.id == first.id
  end

  test "el mismo client_message_id en otro canal es un conflicto", ctx do
    client_id = Ecto.UUID.generate()
    other = channel_fixture(ctx.group, ctx.owner, %{"name" => "otro"})

    assert {:ok, _, :created} = send_message(ctx, "hola", %{"client_message_id" => client_id})

    assert {:error, :client_message_id_conflict} =
             Messaging.create_message(other, ctx.group, ctx.owner, %{
               "client_message_id" => client_id,
               "content" => "hola"
             })
  end

  test "valida contenido, client_message_id y reply_to", ctx do
    assert {:error, changeset} = send_message(ctx, "")
    assert %{content: [_]} = errors_on(changeset)

    assert {:error, changeset} = send_message(ctx, String.duplicate("a", 4_001))
    assert %{content: [_]} = errors_on(changeset)

    assert {:error, changeset} =
             Messaging.create_message(ctx.channel, ctx.group, ctx.owner, %{"content" => "x"})

    assert %{client_message_id: [_]} = errors_on(changeset)

    assert {:error, changeset} = send_message(ctx, "x", %{"reply_to" => TimeUUID.generate()})
    assert %{reply_to: [_]} = errors_on(changeset)
  end

  test "responder a un mensaje del mismo canal (CU-17)", ctx do
    {:ok, original, :created} = send_message(ctx, "pregunta")

    assert {:ok, reply, :created} = send_message(ctx, "respuesta", %{"reply_to" => original.id})
    assert reply.reply_to == original.id
  end

  test "historial paginado por cursor, del más reciente al más antiguo", ctx do
    ids = for i <- 1..5, do: elem(send_message(ctx, "m#{i}"), 1).id

    assert {:ok, %{messages: page1, next_cursor: cursor}} =
             Messaging.list_messages(ctx.channel, %{"limit" => "2"})

    assert Enum.map(page1, & &1.content) == ["m5", "m4"]
    assert cursor == Enum.at(ids, 3)

    assert {:ok, %{messages: page2, next_cursor: cursor2}} =
             Messaging.list_messages(ctx.channel, %{"limit" => "2", "before" => cursor})

    assert Enum.map(page2, & &1.content) == ["m3", "m2"]

    assert {:ok, %{messages: [%{content: "m1"}], next_cursor: nil}} =
             Messaging.list_messages(ctx.channel, %{"limit" => "2", "before" => cursor2})
  end

  test "valida limit y cursor", ctx do
    assert {:error, changeset} = Messaging.list_messages(ctx.channel, %{"limit" => "101"})
    assert %{limit: [_]} = errors_on(changeset)

    assert {:error, changeset} =
             Messaging.list_messages(ctx.channel, %{"before" => Ecto.UUID.generate()})

    assert %{before: [_]} = errors_on(changeset)
  end

  test "no admite mensajes en canales de voz ni en grupos archivados", ctx do
    voice = channel_fixture(ctx.group, ctx.owner, %{"name" => "sala", "type" => "voice"})
    attrs = %{"client_message_id" => Ecto.UUID.generate(), "content" => "x"}

    assert {:error, :not_text_channel} =
             Messaging.create_message(voice, ctx.group, ctx.owner, attrs)

    archived = %{ctx.group | archived_at: DateTime.utc_now()}

    assert {:error, :group_archived} =
             Messaging.create_message(ctx.channel, archived, ctx.owner, attrs)
  end
end
