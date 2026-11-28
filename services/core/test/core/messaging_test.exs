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

  describe "attachments (CU-23)" do
    test "stores a message with text and a file", ctx do
      file_id = Ecto.UUID.generate()

      assert {:ok, message, :created} = send_message(ctx, "mira esto", %{"file_id" => file_id})
      assert message.file_id == file_id
      assert message.content == "mira esto"

      assert {:ok, %{messages: [stored]}} = Messaging.list_messages(ctx.channel, %{})
      assert stored.file_id == file_id
      assert stored.content == "mira esto"
    end

    test "text is optional when there is a file", ctx do
      file_id = Ecto.UUID.generate()

      assert {:ok, message, :created} = send_message(ctx, "", %{"file_id" => file_id})
      assert message.content == ""
      assert message.file_id == file_id

      assert {:ok, message, :created} =
               Messaging.create_message(ctx.channel, ctx.group, ctx.owner, %{
                 "client_message_id" => Ecto.UUID.generate(),
                 "file_id" => file_id
               })

      assert message.content == ""
    end

    test "a message needs text or a file", ctx do
      assert {:error, changeset} = send_message(ctx, "", %{"file_id" => nil})
      assert %{content: [_]} = errors_on(changeset)

      assert {:error, changeset} =
               Messaging.create_message(ctx.channel, ctx.group, ctx.owner, %{
                 "client_message_id" => Ecto.UUID.generate()
               })

      assert %{content: [_]} = errors_on(changeset)
    end

    test "the file id must be a uuid", ctx do
      assert {:error, changeset} = send_message(ctx, "hola", %{"file_id" => "no-es-uuid"})
      assert %{file_id: [_]} = errors_on(changeset)
    end

    test "text is still capped at 4000 characters with a file", ctx do
      assert {:error, changeset} =
               send_message(ctx, String.duplicate("a", 4_001), %{
                 "file_id" => Ecto.UUID.generate()
               })

      assert %{content: [_]} = errors_on(changeset)
    end

    test "a retry returns the stored message and keeps its file", ctx do
      client_id = Ecto.UUID.generate()
      file_id = Ecto.UUID.generate()

      assert {:ok, first, :created} =
               send_message(ctx, "foto", %{"client_message_id" => client_id, "file_id" => file_id})

      assert {:ok, again, :duplicate} =
               send_message(ctx, "foto", %{
                 "client_message_id" => client_id,
                 "file_id" => Ecto.UUID.generate()
               })

      assert again.id == first.id
      assert again.file_id == file_id
    end

    test "a deleted message serves neither text nor file", ctx do
      {:ok, message, :created} =
        send_message(ctx, "secreto", %{"file_id" => Ecto.UUID.generate()})

      {:ok, datetime} = TimeUUID.to_datetime(message.id)

      {:ok, _} =
        Core.Scylla.execute(
          "UPDATE messages_by_channel SET deleted = true WHERE channel_id = ? AND bucket = ? AND message_id = ?",
          [ctx.channel.id, TimeUUID.bucket(datetime), message.id]
        )

      assert {:ok, %{messages: [stored]}} = Messaging.list_messages(ctx.channel, %{})
      assert stored.deleted
      assert stored.content == nil
      assert stored.file_id == nil
    end
  end

  describe "editing and deleting own messages (CU-19)" do
    test "the author edits and the history shows the new text as edited", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")

      assert {:ok, edited} =
               Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, %{
                 "content" => "hola a todos"
               })

      assert edited.content == "hola a todos"
      assert %DateTime{} = edited.edited_at

      assert {:ok, %{messages: [stored]}} = Messaging.list_messages(ctx.channel, %{})
      assert stored.content == "hola a todos"
      assert stored.edited_at
    end

    test "someone else cannot edit or delete it (EX-01)", ctx do
      {:ok, message, :created} = send_message(ctx, "mio")
      other = user_id()

      assert {:error, :forbidden} =
               Messaging.edit_message(ctx.channel, ctx.group, other, message.id, %{
                 "content" => "x"
               })

      assert {:error, :forbidden} =
               Messaging.delete_message(ctx.channel, ctx.group, other, message.id)

      assert {:ok, %{messages: [%{content: "mio", edited_at: nil, deleted: false}]}} =
               Messaging.list_messages(ctx.channel, %{})
    end

    test "empty, missing or too long content is a validation error (EX-02)", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")

      for attrs <- [%{"content" => ""}, %{}, %{"content" => String.duplicate("a", 4_001)}] do
        assert {:error, changeset} =
                 Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, attrs)

        assert %{content: [_ | _]} = errors_on(changeset)
      end
    end

    test "a message with a file may be edited down to empty text", ctx do
      {:ok, message, :created} = send_message(ctx, "foto", %{"file_id" => Ecto.UUID.generate()})

      assert {:ok, %{content: ""}} =
               Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, %{
                 "content" => ""
               })
    end

    test "unknown, malformed and other-channel ids are not found", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")
      other = channel_fixture(ctx.group, ctx.owner, %{"name" => "otro"})

      for id <- [TimeUUID.generate(), "no-es-uuid", Ecto.UUID.generate()] do
        assert {:error, :not_found} =
                 Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, id, %{"content" => "x"})

        assert {:error, :not_found} =
                 Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, id)
      end

      assert {:error, :not_found} =
               Messaging.edit_message(other, ctx.group, ctx.owner, message.id, %{"content" => "x"})
    end

    test "deleting leaves a notice without text or file", ctx do
      {:ok, message, :created} =
        send_message(ctx, "secreto", %{"file_id" => Ecto.UUID.generate()})

      assert {:ok, deleted} =
               Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, message.id)

      assert %{deleted: true, content: nil, file_id: nil, deleted_reason: "author"} = deleted

      assert {:ok, %{messages: [stored]}} = Messaging.list_messages(ctx.channel, %{})
      assert %{deleted: true, content: nil, file_id: nil, deleted_reason: "author"} = stored
    end

    test "a deleted message can be neither edited nor deleted again", ctx do
      {:ok, message, :created} = send_message(ctx, "adios")
      {:ok, _} = Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, message.id)

      assert {:error, :not_found} =
               Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, %{
                 "content" => "x"
               })

      assert {:error, :not_found} =
               Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, message.id)
    end

    test "an archived group rejects both", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")
      archived = %{ctx.group | archived_at: DateTime.utc_now()}

      assert {:error, :group_archived} =
               Messaging.edit_message(ctx.channel, archived, ctx.owner, message.id, %{
                 "content" => "x"
               })

      assert {:error, :group_archived} =
               Messaging.delete_message(ctx.channel, archived, ctx.owner, message.id)
    end

    test "concurrent edit and delete never resurrect the message", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")

      results =
        [
          Task.async(fn ->
            Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, %{
              "content" => "nuevo"
            })
          end),
          Task.async(fn ->
            Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, message.id)
          end)
        ]
        |> Task.await_many(10_000)

      assert Enum.any?(results, &match?({:ok, %{deleted: true}}, &1))

      assert {:ok, %{messages: [%{deleted: true, content: nil}]}} =
               Messaging.list_messages(ctx.channel, %{})
    end
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
