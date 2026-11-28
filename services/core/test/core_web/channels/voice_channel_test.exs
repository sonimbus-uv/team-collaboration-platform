defmodule CoreWeb.VoiceChannelTest do
  use CoreWeb.ChannelCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers

  alias Core.Realtime
  alias CoreWeb.{Presence, UserSocket}

  setup do
    Process.flag(:trap_exit, true)
    owner = user_id()
    group = group_fixture(owner)
    voice = channel_fixture(group, owner, %{"name" => "sala", "type" => "voice"})
    text = channel_fixture(group, owner, %{"name" => "texto"})
    {:ok, _} = Core.Profiles.put_display_name(owner, %{"display_name" => "Ana"})
    %{owner: owner, group: group, voice: voice, text: text}
  end

  defp connect_as(user) do
    {:ok, socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})
    socket
  end

  defp join_voice(user, channel, payload \\ %{}) do
    subscribe_and_join(connect_as(user), "voice:#{channel.id}", payload)
  end

  defp eventually(fun, attempts \\ 40) do
    cond do
      fun.() ->
        :ok

      attempts == 0 ->
        flunk("condition not met")

      true ->
        Process.sleep(50)
        eventually(fun, attempts - 1)
    end
  end

  defp participants(channel), do: Presence.list("voice:#{channel.id}")

  test "joins and receives its own state with listening-only defaults", ctx do
    {:ok, _, _socket} = join_voice(ctx.owner, ctx.voice)
    owner = ctx.owner

    assert_push "presence_state", %{
      ^owner => %{metas: [%{display_name: "Ana", mic: false, camera: false, joined_at: joined}]}
    }

    assert {:ok, _, _} = DateTime.from_iso8601(joined)
  end

  test "joins with the microphone and camera the client asks for", ctx do
    {:ok, _, _} = join_voice(ctx.owner, ctx.voice, %{"mic" => true, "camera" => true})

    eventually(fn ->
      match?(%{metas: [%{mic: true, camera: true}]}, participants(ctx.voice)[ctx.owner])
    end)
  end

  test "a second participant appears for the first one", ctx do
    other = user_id()
    member_fixture(ctx.group, other, :member)
    {:ok, _, _} = join_voice(ctx.owner, ctx.voice)
    {:ok, _, _} = join_voice(other, ctx.voice)

    eventually(fn -> map_size(participants(ctx.voice)) == 2 end)
    assert_push "presence_diff", %{joins: joins}
    assert is_map(joins)
  end

  test "voice.update changes the microphone and camera state", ctx do
    {:ok, _, socket} = join_voice(ctx.owner, ctx.voice)

    ref = push(socket, "voice.update", %{"mic" => true})
    assert_reply ref, :ok, %{}

    eventually(fn ->
      match?(%{metas: [%{mic: true, camera: false}]}, participants(ctx.voice)[ctx.owner])
    end)

    ref = push(socket, "voice.update", %{"camera" => true, "mic" => false})
    assert_reply ref, :ok, %{}

    eventually(fn ->
      match?(%{metas: [%{mic: false, camera: true}]}, participants(ctx.voice)[ctx.owner])
    end)
  end

  test "voice.update rejects non-boolean values and empty updates", ctx do
    {:ok, _, socket} = join_voice(ctx.owner, ctx.voice)

    ref = push(socket, "voice.update", %{"mic" => "yes"})
    assert_reply ref, :error, %{type: "validation-error", errors: %{mic: ["must be a boolean"]}}

    ref = push(socket, "voice.update", %{})
    assert_reply ref, :error, %{type: "validation-error"}
  end

  test "leaving removes the participant", ctx do
    {:ok, _, socket} = join_voice(ctx.owner, ctx.voice)
    eventually(fn -> map_size(participants(ctx.voice)) == 1 end)

    leave(socket)

    eventually(fn -> participants(ctx.voice) == %{} end)
  end

  test "a lost connection removes the participant", ctx do
    {:ok, _, socket} = join_voice(ctx.owner, ctx.voice)
    eventually(fn -> map_size(participants(ctx.voice)) == 1 end)

    Process.exit(socket.channel_pid, :kill)

    eventually(fn -> participants(ctx.voice) == %{} end)
  end

  test "a non member cannot join", ctx do
    assert {:error, %{reason: "not_found"}} = join_voice(user_id(), ctx.voice)
  end

  test "a role below min_role cannot join", ctx do
    admin_only =
      channel_fixture(ctx.group, ctx.owner, %{
        "name" => "mesa",
        "type" => "voice",
        "min_role" => "admin"
      })

    member = user_id()
    member_fixture(ctx.group, member, :member)

    assert {:error, %{reason: "not_found"}} = join_voice(member, admin_only)
  end

  test "a text channel is not a voice room", ctx do
    assert {:error, %{reason: "not_found"}} = join_voice(ctx.owner, ctx.text)
  end

  test "an archived group rejects the join", ctx do
    ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Core.Repo.update!()

    assert {:error, %{reason: "group_archived"}} = join_voice(ctx.owner, ctx.voice)
  end

  test "a non boolean join payload is rejected", ctx do
    assert {:error, %{reason: "invalid_payload"}} =
             join_voice(ctx.owner, ctx.voice, %{"mic" => "on"})
  end

  test "an expired token cannot join", ctx do
    expired =
      Phoenix.Socket.assign(
        connect_as(ctx.owner),
        :token_expires_at,
        System.system_time(:second) - 1
      )

    assert {:error, %{reason: "token_expired"}} =
             subscribe_and_join(expired, "voice:#{ctx.voice.id}", %{})
  end

  test "expulsion closes the room for that user only", ctx do
    other = user_id()
    member_fixture(ctx.group, other, :member)
    {:ok, _, owner_socket} = join_voice(ctx.owner, ctx.voice)
    {:ok, _, other_socket} = join_voice(other, ctx.voice)
    owner_ref = Process.monitor(owner_socket.channel_pid)
    other_ref = Process.monitor(other_socket.channel_pid)
    eventually(fn -> map_size(participants(ctx.voice)) == 2 end)

    Realtime.disconnect_user_from_group(ctx.group.id, other)

    assert_receive {:DOWN, ^other_ref, :process, _, {:shutdown, :access_revoked}}
    refute_receive {:DOWN, ^owner_ref, :process, _, _}
    eventually(fn -> Map.keys(participants(ctx.voice)) == [ctx.owner] end)
  end

  test "unknown events get an error reply", ctx do
    {:ok, _, socket} = join_voice(ctx.owner, ctx.voice)
    ref = push(socket, "nope", %{})
    assert_reply ref, :error, %{type: "unknown-event"}
  end
end
