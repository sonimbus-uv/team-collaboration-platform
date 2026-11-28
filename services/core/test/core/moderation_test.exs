defmodule Core.ModerationTest do
  use CoreWeb.ChannelCase, async: true

  import Ecto.Query
  import Core.Fixtures
  import Core.AccessTokenHelpers

  alias Core.Community
  alias Core.Community.{Ban, ModerationAction}
  alias Core.Repo
  alias CoreWeb.{Endpoint, Presence, UserSocket}

  setup do
    Process.flag(:trap_exit, true)
    owner = user_id()
    group = group_fixture(owner)
    moderator = user_id()
    member = user_id()
    other = user_id()
    member_fixture(group, moderator, :moderator)
    member_fixture(group, member, :member)
    member_fixture(group, other, :member)
    text = channel_fixture(group, owner)
    voice = channel_fixture(group, owner, %{"name" => "sala", "type" => "voice"})

    %{
      owner: owner,
      moderator: moderator,
      member: member,
      other: other,
      group: group,
      text: text,
      voice: voice
    }
  end

  defp member?(group, user),
    do: match?({:ok, _, _}, Community.get_group_for_member(group.id, user))

  defp actions(group), do: Repo.all(from a in ModerationAction, where: a.group_id == ^group.id)

  defp join_topic(user, topic) do
    {:ok, socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})
    {:ok, _, socket} = subscribe_and_join(socket, topic, %{})
    {socket, Process.monitor(socket.channel_pid)}
  end

  describe "expel_member/4" do
    test "a moderator expels a member, who can come back with an invitation", ctx do
      assert :ok = Community.expel_member(ctx.group.id, ctx.moderator, ctx.member, %{})
      refute member?(ctx.group, ctx.member)

      invitation = invitation_fixture(ctx.group, ctx.owner)
      assert {:ok, :joined, _group, :member} = Community.join_group(invitation.code, ctx.member)
    end

    test "the action is recorded with who, whom, what and why", ctx do
      :ok =
        Community.expel_member(ctx.group.id, ctx.moderator, ctx.member, %{"reason" => "  Spam "})

      assert [%ModerationAction{} = action] = actions(ctx.group)
      assert action.actor_id == ctx.moderator
      assert action.target_user_id == ctx.member
      assert action.action == :expel
      assert action.reason == "Spam"
      assert DateTime.diff(DateTime.utc_now(), action.created_at) < 60
    end

    test "the affected user is notified on the personal topic", ctx do
      Endpoint.subscribe("user:#{ctx.member}")
      group_id = ctx.group.id

      :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{"reason" => "Spam"})

      assert_receive %Phoenix.Socket.Broadcast{
        event: "moderation.applied",
        payload: %{
          type: "moderation.applied",
          version: 1,
          payload: %{group_id: ^group_id, action: :expel, reason: "Spam"}
        }
      }
    end

    test "text channels and the voice room close for that user only", ctx do
      {_, text_ref} = join_topic(ctx.member, "channel:#{ctx.text.id}")
      {_, voice_ref} = join_topic(ctx.member, "voice:#{ctx.voice.id}")
      {_, other_ref} = join_topic(ctx.other, "channel:#{ctx.text.id}")

      :ok = Community.expel_member(ctx.group.id, ctx.moderator, ctx.member, %{})

      assert_receive {:DOWN, ^text_ref, :process, _, {:shutdown, :access_revoked}}
      assert_receive {:DOWN, ^voice_ref, :process, _, {:shutdown, :access_revoked}}
      refute_receive {:DOWN, ^other_ref, :process, _, _}
      refute Map.has_key?(Presence.list("voice:#{ctx.voice.id}"), ctx.member)
    end

    test "the actor needs a strictly higher role (CU-15 EX-01)", ctx do
      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, ctx.moderator, ctx.owner, %{})

      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, ctx.moderator, ctx.moderator, %{})

      other_admin = user_id()
      member_fixture(ctx.group, other_admin, :admin)

      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, ctx.owner, other_admin, %{})

      assert member?(ctx.group, ctx.owner)
      assert member?(ctx.group, other_admin)
      assert actions(ctx.group) == []
    end

    test "a moderator cannot be expelled by another moderator but can by an admin", ctx do
      second = user_id()
      member_fixture(ctx.group, second, :moderator)

      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, second, ctx.moderator, %{})

      assert :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.moderator, %{})
    end

    test "members and moderators without the permission are forbidden", ctx do
      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, ctx.member, ctx.other, %{})

      {:ok, _} =
        Community.update_role_permissions(ctx.group.id, ctx.owner, "moderator", %{
          "moderate" => false
        })

      assert {:error, :forbidden} =
               Community.expel_member(ctx.group.id, ctx.moderator, ctx.member, %{})
    end

    test "strangers and unknown targets get not_found", ctx do
      assert {:error, :not_found} =
               Community.expel_member(ctx.group.id, user_id(), ctx.member, %{})

      assert {:error, :not_found} =
               Community.expel_member(ctx.group.id, ctx.owner, user_id(), %{})

      assert {:error, :not_found} = Community.expel_member(ctx.group.id, ctx.owner, "nope", %{})
    end

    test "reasons are trimmed, blank means none and long ones are rejected", ctx do
      assert {:error, %Ecto.Changeset{}} =
               Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{
                 "reason" => String.duplicate("x", 501)
               })

      assert {:error, %Ecto.Changeset{}} =
               Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{"reason" => 7})

      assert member?(ctx.group, ctx.member)

      :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{"reason" => "   "})
      assert [%{reason: nil}] = actions(ctx.group)
    end

    test "an archived group can still be moderated", ctx do
      ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Repo.update!()

      assert :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{})
    end
  end

  describe "ban_member/4, list_bans/2 and unban_member/3" do
    test "a ban removes the member and blocks every way back", ctx do
      assert :ok =
               Community.ban_member(ctx.group.id, ctx.moderator, ctx.member, %{
                 "reason" => "Acoso"
               })

      refute member?(ctx.group, ctx.member)

      invitation = invitation_fixture(ctx.group, ctx.owner)
      assert {:error, :banned} = Community.join_group(invitation.code, ctx.member)

      assert %Ban{banned_by: banned_by, reason: "Acoso"} =
               Repo.get_by!(Ban, group_id: ctx.group.id, user_id: ctx.member)

      assert banned_by == ctx.moderator
    end

    test "a ban closes channels and voice like an expulsion", ctx do
      {_, text_ref} = join_topic(ctx.member, "channel:#{ctx.text.id}")
      {_, voice_ref} = join_topic(ctx.member, "voice:#{ctx.voice.id}")

      :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.member, %{})

      assert_receive {:DOWN, ^text_ref, :process, _, {:shutdown, :access_revoked}}
      assert_receive {:DOWN, ^voice_ref, :process, _, {:shutdown, :access_revoked}}
    end

    test "banning follows the same role rules", ctx do
      assert {:error, :forbidden} =
               Community.ban_member(ctx.group.id, ctx.moderator, ctx.owner, %{})

      assert {:error, :forbidden} = Community.ban_member(ctx.group.id, ctx.member, ctx.other, %{})
      assert Repo.aggregate(Ban, :count) == 0
    end

    test "moderators list the banned users with their names, most recent first", ctx do
      {:ok, _} = Core.Profiles.put_display_name(ctx.member, %{"display_name" => "Marta"})
      :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.other, %{})
      :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.member, %{"reason" => "Spam"})

      assert {:ok, [first, second]} = Community.list_bans(ctx.group.id, ctx.moderator)
      assert %{user_id: user, display_name: "Marta", reason: "Spam", banned_by: banner} = first
      assert user == ctx.member
      assert banner == ctx.owner
      assert second.user_id == ctx.other
      refute Map.has_key?(first, :account_status)
    end

    test "only moderators see the list", ctx do
      assert {:error, :forbidden} = Community.list_bans(ctx.group.id, ctx.member)
      assert {:error, :not_found} = Community.list_bans(ctx.group.id, user_id())
    end

    test "unbanning lets the user return with an invitation and is recorded", ctx do
      :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.member, %{})

      assert :ok = Community.unban_member(ctx.group.id, ctx.moderator, ctx.member)
      assert {:ok, []} = Community.list_bans(ctx.group.id, ctx.owner)
      refute member?(ctx.group, ctx.member)

      invitation = invitation_fixture(ctx.group, ctx.owner)
      assert {:ok, :joined, _, :member} = Community.join_group(invitation.code, ctx.member)

      assert [:ban, :unban] ==
               ctx.group |> actions() |> Enum.sort_by(& &1.created_at) |> Enum.map(& &1.action)
    end

    test "unbanning someone who is not banned is not_found and members cannot unban", ctx do
      assert {:error, :not_found} = Community.unban_member(ctx.group.id, ctx.owner, ctx.member)
      assert {:error, :not_found} = Community.unban_member(ctx.group.id, ctx.owner, "nope")

      :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.other, %{})
      assert {:error, :forbidden} = Community.unban_member(ctx.group.id, ctx.member, ctx.other)
    end
  end

  describe "moderation_actions" do
    test "rows can be neither updated nor deleted", ctx do
      :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{})
      [action] = actions(ctx.group)

      assert_raise Postgrex.Error, ~r/append-only/, fn ->
        action |> Ecto.Changeset.change(reason: "editado") |> Repo.update!()
      end
    end
  end
end
