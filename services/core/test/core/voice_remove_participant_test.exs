defmodule Core.VoiceRemoveParticipantTest do
  use Core.DataCase, async: false

  import Core.Fixtures

  alias Core.{Community, FakeLiveKit, Voice}

  @api_key "test_livekit_key"
  @api_secret "test_livekit_secret_with_at_least_32_chars"
  @path "/twirp/livekit.RoomService/RemoveParticipant"

  setup do
    previous = Application.get_env(:core, Core.Voice)

    on_exit(fn ->
      if previous,
        do: Application.put_env(:core, Core.Voice, previous),
        else: Application.delete_env(:core, Core.Voice)
    end)

    :ok
  end

  defp configure(url),
    do:
      Application.put_env(:core, Core.Voice, api_key: @api_key, api_secret: @api_secret, url: url)

  defp claims_of(authorization) do
    "Bearer " <> jwt = authorization
    [header, payload, signature] = String.split(jwt, ".")
    expected = :crypto.mac(:hmac, :sha256, @api_secret, header <> "." <> payload)
    assert Base.url_decode64!(signature, padding: false) == expected
    Jason.decode!(Base.url_decode64!(payload, padding: false))
  end

  describe "remove_participant/3" do
    test "asks LiveKit to remove the participant with a short-lived admin token" do
      configure(FakeLiveKit.start(self()))
      channel_id = user_id()
      user = user_id()
      now = System.system_time(:second)

      assert :ok = Voice.remove_participant(channel_id, user, now: now)

      assert_receive {:livekit_request, "POST", @path, authorization, body}
      assert Jason.decode!(body) == %{"room" => channel_id, "identity" => user}

      claims = claims_of(authorization)
      assert claims["iss"] == @api_key
      assert claims["video"] == %{"roomAdmin" => true, "room" => channel_id}
      assert claims["exp"] - now <= 60
    end

    test "treats a participant that is not in the room (404) as removed" do
      configure(FakeLiveKit.start(self(), 404))
      assert :ok = Voice.remove_participant(user_id(), user_id())
    end

    test "reports other failures from LiveKit" do
      configure(FakeLiveKit.start(self(), 500))
      assert {:error, :livekit_failed} = Voice.remove_participant(user_id(), user_id())
    end

    test "reports an unreachable LiveKit" do
      configure("ws://127.0.0.1:1")
      assert {:error, :livekit_failed} = Voice.remove_participant(user_id(), user_id())
    end

    test "reports that voice is not configured" do
      Application.delete_env(:core, Core.Voice)
      assert {:error, :voice_unavailable} = Voice.remove_participant(user_id(), user_id())
    end
  end

  describe "moderation" do
    setup do
      owner = user_id()
      group = group_fixture(owner)
      member = user_id()
      member_fixture(group, member, :member)
      channel_fixture(group, owner)
      voice_a = channel_fixture(group, owner, %{"name" => "sala-a", "type" => "voice"})
      voice_b = channel_fixture(group, owner, %{"name" => "sala-b", "type" => "voice"})
      %{owner: owner, group: group, member: member, rooms: Enum.sort([voice_a.id, voice_b.id])}
    end

    defp rooms_requested(count) do
      for _ <- 1..count do
        assert_receive {:livekit_request, "POST", @path, _authorization, body}, 2_000
        Jason.decode!(body)
      end
    end

    test "expelling removes the member from every voice room of the group", ctx do
      configure(FakeLiveKit.start(self()))
      assert :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{})

      requests = rooms_requested(2)
      assert Enum.all?(requests, &(&1["identity"] == ctx.member))
      assert requests |> Enum.map(& &1["room"]) |> Enum.sort() == ctx.rooms
      refute_receive {:livekit_request, _, _, _, _}, 100
    end

    test "banning removes the member from every voice room of the group", ctx do
      configure(FakeLiveKit.start(self()))
      assert :ok = Community.ban_member(ctx.group.id, ctx.owner, ctx.member, %{})

      requests = rooms_requested(2)
      assert requests |> Enum.map(& &1["room"]) |> Enum.sort() == ctx.rooms
    end

    test "moderation succeeds when LiveKit fails", ctx do
      configure(FakeLiveKit.start(self(), 500))
      assert :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{})
      rooms_requested(2)
    end

    test "moderation succeeds when LiveKit is not configured", ctx do
      Application.delete_env(:core, Core.Voice)
      assert :ok = Community.expel_member(ctx.group.id, ctx.owner, ctx.member, %{})
      refute_receive {:livekit_request, _, _, _, _}, 100
    end
  end
end
