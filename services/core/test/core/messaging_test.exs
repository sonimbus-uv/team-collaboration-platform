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
               Messaging.delete_message(ctx.channel, ctx.group, other, :member, message.id)

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
                 Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, id)
      end

      assert {:error, :not_found} =
               Messaging.edit_message(other, ctx.group, ctx.owner, message.id, %{"content" => "x"})
    end

    test "deleting leaves a notice without text or file", ctx do
      {:ok, message, :created} =
        send_message(ctx, "secreto", %{"file_id" => Ecto.UUID.generate()})

      assert {:ok, deleted} =
               Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, message.id)

      assert %{deleted: true, content: nil, file_id: nil, deleted_reason: "author"} = deleted

      assert {:ok, %{messages: [stored]}} = Messaging.list_messages(ctx.channel, %{})
      assert %{deleted: true, content: nil, file_id: nil, deleted_reason: "author"} = stored
    end

    test "a deleted message can be neither edited nor deleted again", ctx do
      {:ok, message, :created} = send_message(ctx, "adios")
      {:ok, _} = Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, message.id)

      assert {:error, :not_found} =
               Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, message.id, %{
                 "content" => "x"
               })

      assert {:error, :not_found} =
               Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, message.id)
    end

    test "an archived group rejects both", ctx do
      {:ok, message, :created} = send_message(ctx, "hola")
      archived = %{ctx.group | archived_at: DateTime.utc_now()}

      assert {:error, :group_archived} =
               Messaging.edit_message(ctx.channel, archived, ctx.owner, message.id, %{
                 "content" => "x"
               })

      assert {:error, :group_archived} =
               Messaging.delete_message(ctx.channel, archived, ctx.owner, :admin, message.id)
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
            Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, message.id)
          end)
        ]
        |> Task.await_many(10_000)

      assert Enum.any?(results, &match?({:ok, %{deleted: true}}, &1))

      assert {:ok, %{messages: [%{deleted: true, content: nil}]}} =
               Messaging.list_messages(ctx.channel, %{})
    end
  end

  describe "reply preview (CU-17)" do
    test "a reply carries the original's author and text", ctx do
      {:ok, _} = Core.Profiles.put_display_name(ctx.owner, %{"display_name" => "Ana"})
      {:ok, original, :created} = send_message(ctx, "pregunta")

      assert {:ok, reply, :created} = send_message(ctx, "respuesta", %{"reply_to" => original.id})

      assert %{
               id: original_id,
               unavailable: false,
               author: %{id: author_id, display_name: "Ana"},
               content: "pregunta"
             } = reply.reply_preview

      assert original_id == original.id
      assert author_id == ctx.owner

      assert {:ok, %{messages: [stored, _]}} = Messaging.list_messages(ctx.channel, %{})
      assert stored.reply_preview == reply.reply_preview
    end

    test "a message that is not a reply has no preview", ctx do
      assert {:ok, message, :created} = send_message(ctx, "suelto")
      assert message.reply_preview == nil
    end

    test "the preview keeps only the first 100 characters", ctx do
      {:ok, original, :created} = send_message(ctx, String.duplicate("a", 150))
      {:ok, reply, :created} = send_message(ctx, "ok", %{"reply_to" => original.id})

      assert String.length(reply.reply_preview.content) == 100
    end

    test "a deleted original becomes unavailable (EX-01)", ctx do
      {:ok, original, :created} = send_message(ctx, "borrame")
      {:ok, reply, :created} = send_message(ctx, "respuesta", %{"reply_to" => original.id})
      {:ok, _} = Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, original.id)

      assert {:ok, %{messages: [stored, _deleted]}} = Messaging.list_messages(ctx.channel, %{})
      assert stored.id == reply.id
      assert stored.reply_preview == %{id: original.id, unavailable: true}
    end

    test "editing a reply keeps its preview", ctx do
      {:ok, original, :created} = send_message(ctx, "pregunta")
      {:ok, reply, :created} = send_message(ctx, "respuesta", %{"reply_to" => original.id})

      assert {:ok, edited} =
               Messaging.edit_message(ctx.channel, ctx.group, ctx.owner, reply.id, %{
                 "content" => "mejor respuesta"
               })

      assert %{unavailable: false, content: "pregunta"} = edited.reply_preview
    end
  end

  describe "history around a message (CU-20 FA-01)" do
    setup ctx do
      ids = for i <- 1..7, do: elem(send_message(ctx, "m#{i}"), 1).id
      %{ids: ids}
    end

    defp contents(page), do: Enum.map(page.messages, & &1.content)

    test "centers the page on the message, newest first", ctx do
      around = Enum.at(ctx.ids, 3)

      assert {:ok, page} =
               Messaging.list_messages(ctx.channel, %{"around" => around, "limit" => "5"})

      assert contents(page) == ["m6", "m5", "m4", "m3", "m2"]
      assert page.next_cursor == Enum.at(ctx.ids, 1)

      assert {:ok, older} =
               Messaging.list_messages(ctx.channel, %{
                 "before" => page.next_cursor,
                 "limit" => "5"
               })

      assert contents(older) == ["m1"]
      assert older.next_cursor == nil
    end

    test "around the newest message fills the page with older ones", ctx do
      assert {:ok, page} =
               Messaging.list_messages(ctx.channel, %{
                 "around" => List.last(ctx.ids),
                 "limit" => "5"
               })

      assert contents(page) == ["m7", "m6", "m5", "m4", "m3"]
    end

    test "around the oldest message returns what exists and no cursor", ctx do
      assert {:ok, page} =
               Messaging.list_messages(ctx.channel, %{"around" => hd(ctx.ids), "limit" => "5"})

      assert contents(page) == ["m3", "m2", "m1"]
      assert page.next_cursor == nil
    end

    test "a limit of one returns just the message", ctx do
      assert {:ok, page} =
               Messaging.list_messages(ctx.channel, %{
                 "around" => Enum.at(ctx.ids, 2),
                 "limit" => "1"
               })

      assert contents(page) == ["m3"]
    end

    test "an id that is not in the channel is not found", ctx do
      assert {:error, :not_found} =
               Messaging.list_messages(ctx.channel, %{"around" => TimeUUID.generate()})
    end

    test "invalid ids and the combination with before are validation errors", ctx do
      assert {:error, changeset} =
               Messaging.list_messages(ctx.channel, %{"around" => Ecto.UUID.generate()})

      assert %{around: [_]} = errors_on(changeset)

      assert {:error, changeset} =
               Messaging.list_messages(ctx.channel, %{
                 "around" => hd(ctx.ids),
                 "before" => List.last(ctx.ids)
               })

      assert %{around: [_]} = errors_on(changeset)
    end

    test "newer messages are found across month boundaries", ctx do
      last_month = DateTime.utc_now() |> DateTime.add(-35 * 86_400)
      old_id = TimeUUID.generate(last_month)

      {:ok, _} =
        Core.Scylla.execute(
          "INSERT INTO messages_by_channel (channel_id, bucket, message_id, author_id, client_message_id, content, deleted) VALUES (?, ?, ?, ?, ?, ?, false)",
          [
            ctx.channel.id,
            TimeUUID.bucket(last_month),
            old_id,
            ctx.owner,
            Ecto.UUID.generate(),
            "viejo"
          ]
        )

      assert {:ok, page} =
               Messaging.list_messages(ctx.channel, %{"around" => old_id, "limit" => "4"})

      assert contents(page) == ["m2", "m1", "viejo"]
    end
  end

  describe "reactions (CU-18)" do
    setup ctx do
      {:ok, message, :created} = send_message(ctx, "hola")
      %{message: message, guest: user_id()}
    end

    defp react(ctx, user, message, emoji \\ "👍") do
      Messaging.toggle_reaction(ctx.channel, ctx.group, user, message.id, emoji)
    end

    test "adds a reaction and counts it", ctx do
      assert {:ok, %{emoji: "👍", count: 1, active: true, user_id: user_id}} =
               react(ctx, ctx.owner, ctx.message)

      assert user_id == ctx.owner
    end

    test "repeating the same reaction removes it (FA-01) and a third time adds it back", ctx do
      assert {:ok, %{count: 1, active: true}} = react(ctx, ctx.owner, ctx.message)
      assert {:ok, %{count: 0, active: false}} = react(ctx, ctx.owner, ctx.message)
      assert {:ok, %{count: 1, active: true}} = react(ctx, ctx.owner, ctx.message)
    end

    test "each user counts once per emoji and emojis are independent", ctx do
      assert {:ok, %{count: 1}} = react(ctx, ctx.owner, ctx.message)
      assert {:ok, %{count: 2}} = react(ctx, ctx.guest, ctx.message)
      assert {:ok, %{count: 1, emoji: "❤️"}} = react(ctx, ctx.guest, ctx.message, "❤️")
    end

    test "the history lists reactions sorted, with reacted_by_me for the reader", ctx do
      {:ok, _} = react(ctx, ctx.owner, ctx.message)
      {:ok, _} = react(ctx, ctx.guest, ctx.message)
      {:ok, _} = react(ctx, ctx.guest, ctx.message, "🎉")
      {:ok, _} = react(ctx, ctx.guest, ctx.message, "😀")
      {:ok, _} = react(ctx, ctx.guest, ctx.message, "😀")

      assert {:ok, %{messages: [mine]}} = Messaging.list_messages(ctx.channel, %{}, ctx.owner)

      assert mine.reactions == [
               %{emoji: "🎉", count: 1, reacted_by_me: false},
               %{emoji: "👍", count: 2, reacted_by_me: true}
             ]

      assert {:ok, %{messages: [theirs]}} = Messaging.list_messages(ctx.channel, %{}, ctx.guest)
      assert Enum.all?(theirs.reactions, & &1.reacted_by_me)
      assert {:ok, %{messages: [anonymous]}} = Messaging.list_messages(ctx.channel, %{})
      refute Enum.any?(anonymous.reactions, & &1.reacted_by_me)
    end

    test "a deleted message rejects reactions and lists none (EX-01)", ctx do
      {:ok, _} = react(ctx, ctx.owner, ctx.message)

      {:ok, _} =
        Messaging.delete_message(ctx.channel, ctx.group, ctx.owner, :admin, ctx.message.id)

      assert {:error, :message_deleted} = react(ctx, ctx.guest, ctx.message)

      assert {:ok, %{messages: [%{deleted: true, reactions: []}]}} =
               Messaging.list_messages(ctx.channel, %{})
    end

    test "unknown messages are not found", ctx do
      assert {:error, :not_found} = react(ctx, ctx.owner, %{id: TimeUUID.generate()})
      assert {:error, :not_found} = react(ctx, ctx.owner, %{id: "no-es-uuid"})
    end

    test "only a single emoji is accepted", ctx do
      for emoji <- ["", "a", "7", " ", "👍👍", "ok", String.duplicate("👍", 9), nil, 42] do
        assert {:error, %Ecto.Changeset{} = changeset} = react(ctx, ctx.owner, ctx.message, emoji)
        assert %{emoji: [_]} = errors_on(changeset)
      end

      assert {:ok, _} = react(ctx, ctx.owner, ctx.message, "1️⃣")
      assert {:ok, _} = react(ctx, ctx.owner, ctx.message, "🇲🇽")
    end

    test "an archived group rejects reactions", ctx do
      archived = %{ctx.group | archived_at: DateTime.utc_now()}

      assert {:error, :group_archived} =
               Messaging.toggle_reaction(ctx.channel, archived, ctx.owner, ctx.message.id, "👍")
    end

    test "concurrent toggles by one user never duplicate the reaction", ctx do
      results =
        1..3
        |> Enum.map(fn _ -> Task.async(fn -> react(ctx, ctx.owner, ctx.message) end) end)
        |> Task.await_many(15_000)

      applied = Enum.count(results, &match?({:ok, _}, &1))
      assert applied >= 1

      {:ok, %{messages: [message]}} = Messaging.list_messages(ctx.channel, %{}, ctx.owner)
      expected = if rem(applied, 2) == 1, do: 1, else: 0
      assert Enum.sum(for %{count: count} <- message.reactions, do: count) == expected
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
