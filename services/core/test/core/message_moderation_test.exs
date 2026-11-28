defmodule Core.MessageModerationTest do
  use Core.DataCase, async: true

  import Core.Fixtures

  alias Core.{Community, Messaging}
  alias Core.Community.{Member, ModerationAction}
  alias CoreWeb.Endpoint

  @moduletag :scylla

  setup_all do
    Core.ScyllaHelpers.await_scylla!()
  end

  setup do
    owner = user_id()
    group = group_fixture(owner)
    channel = channel_fixture(group, owner)
    moderator = user_id()
    author = user_id()
    other = user_id()
    member_fixture(group, moderator, :moderator)
    member_fixture(group, author, :member)
    member_fixture(group, other, :member)

    %{
      owner: owner,
      group: group,
      channel: channel,
      moderator: moderator,
      author: author,
      other: other
    }
  end

  defp post_as(ctx, user, content \\ "hola") do
    {:ok, message, :created} =
      Messaging.create_message(ctx.channel, ctx.group, user, %{
        "client_message_id" => Ecto.UUID.generate(),
        "content" => content
      })

    message
  end

  defp delete_as(ctx, user, role, message, attrs \\ %{}) do
    Messaging.delete_message(ctx.channel, ctx.group, user, role, message.id, attrs)
  end

  defp actions(ctx), do: Repo.all(from a in ModerationAction, where: a.group_id == ^ctx.group.id)

  test "a moderator deletes a member's message and it shows as moderated", ctx do
    message = post_as(ctx, ctx.author)

    assert {:ok, %{deleted: true, content: nil, deleted_reason: "moderation"}} =
             delete_as(ctx, ctx.moderator, :moderator, message)

    assert {:ok, %{messages: [%{deleted: true, content: nil, deleted_reason: "moderation"}]}} =
             Messaging.list_messages(ctx.channel, %{})
  end

  test "the action is recorded and the author is notified", ctx do
    message = post_as(ctx, ctx.author)
    Endpoint.subscribe("user:#{ctx.author}")
    group_id = ctx.group.id
    message_id = message.id

    {:ok, _} = delete_as(ctx, ctx.moderator, :moderator, message, %{"reason" => "Ofensivo"})

    assert [%ModerationAction{} = action] = actions(ctx)
    assert action.action == :delete_message
    assert action.actor_id == ctx.moderator
    assert action.target_user_id == ctx.author
    assert action.target_ref == message.id
    assert action.reason == "Ofensivo"

    assert_receive %Phoenix.Socket.Broadcast{
      event: "moderation.applied",
      payload: %{
        type: "moderation.applied",
        version: 1,
        payload: %{
          group_id: ^group_id,
          action: :delete_message,
          message_id: ^message_id,
          reason: "Ofensivo"
        }
      }
    }
  end

  test "the author keeps their membership after a moderated deletion", ctx do
    message = post_as(ctx, ctx.author)
    {:ok, _} = delete_as(ctx, ctx.moderator, :moderator, message)

    assert {:ok, _group, :member} = Community.get_group_for_member(ctx.group.id, ctx.author)
  end

  test "an equal or higher role is protected (EX-01)", ctx do
    admin_message = post_as(ctx, ctx.owner)
    moderator_message = post_as(ctx, ctx.moderator)
    second_moderator = user_id()
    member_fixture(ctx.group, second_moderator, :moderator)

    assert {:error, :forbidden} = delete_as(ctx, ctx.moderator, :moderator, admin_message)
    assert {:error, :forbidden} = delete_as(ctx, second_moderator, :moderator, moderator_message)
    assert {:ok, _} = delete_as(ctx, ctx.owner, :admin, moderator_message)

    other_admin = user_id()
    member_fixture(ctx.group, other_admin, :admin)
    assert {:error, :forbidden} = delete_as(ctx, other_admin, :admin, admin_message)
    assert actions(ctx) |> length() == 1
  end

  test "members and moderators without the permission cannot delete others' messages", ctx do
    message = post_as(ctx, ctx.author)
    assert {:error, :forbidden} = delete_as(ctx, ctx.other, :member, message)

    {:ok, _} =
      Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", %{
        "moderate" => false
      })

    assert {:error, :forbidden} = delete_as(ctx, ctx.moderator, :moderator, message)
    assert {:ok, %{messages: [%{deleted: false}]}} = Messaging.list_messages(ctx.channel, %{})
    assert actions(ctx) == []
  end

  test "a moderator can delete the message of someone who already left", ctx do
    message = post_as(ctx, ctx.author)

    Repo.delete_all(
      from m in Member, where: m.group_id == ^ctx.group.id and m.user_id == ^ctx.author
    )

    assert {:ok, %{deleted_reason: "moderation"}} =
             delete_as(ctx, ctx.moderator, :moderator, message)
  end

  test "an archived group still allows moderation but not the author's own deletion", ctx do
    message = post_as(ctx, ctx.author)
    other_message = post_as(ctx, ctx.author, "otro")
    archived = %{ctx.group | archived_at: DateTime.utc_now()}

    assert {:error, :group_archived} =
             Messaging.delete_message(
               ctx.channel,
               archived,
               ctx.author,
               :member,
               other_message.id
             )

    assert {:ok, %{deleted_reason: "moderation"}} =
             Messaging.delete_message(
               ctx.channel,
               archived,
               ctx.moderator,
               :moderator,
               message.id
             )
  end

  test "an invalid reason leaves the message untouched", ctx do
    message = post_as(ctx, ctx.author)

    assert {:error, %Ecto.Changeset{}} =
             delete_as(ctx, ctx.moderator, :moderator, message, %{
               "reason" => String.duplicate("x", 501)
             })

    assert {:ok, %{messages: [%{deleted: false, content: "hola"}]}} =
             Messaging.list_messages(ctx.channel, %{})
  end

  test "deleting your own message as a moderator is an author deletion without audit", ctx do
    message = post_as(ctx, ctx.moderator)

    assert {:ok, %{deleted_reason: "author"}} = delete_as(ctx, ctx.moderator, :moderator, message)
    assert actions(ctx) == []
  end

  test "an already deleted message is not found", ctx do
    message = post_as(ctx, ctx.author)
    {:ok, _} = delete_as(ctx, ctx.moderator, :moderator, message)

    assert {:error, :not_found} = delete_as(ctx, ctx.owner, :admin, message)
    assert actions(ctx) |> length() == 1
  end
end
