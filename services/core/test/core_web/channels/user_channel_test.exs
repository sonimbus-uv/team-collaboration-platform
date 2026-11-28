defmodule CoreWeb.UserChannelTest do
  use CoreWeb.ChannelCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers

  alias CoreWeb.UserSocket

  setup do
    user = user_id()
    {:ok, socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})
    %{user: user, socket: socket}
  end

  test "joins its own topic", %{user: user, socket: socket} do
    assert {:ok, _, _socket} = subscribe_and_join(socket, "user:#{user}", %{})
  end

  test "rejects another user's topic", %{socket: socket} do
    assert {:error, %{reason: "not_found"}} = subscribe_and_join(socket, "user:#{user_id()}", %{})
  end

  test "rejects a malformed topic id", %{socket: socket} do
    assert {:error, %{reason: "not_found"}} = subscribe_and_join(socket, "user:*", %{})
  end

  test "rejects joining with an expired token", %{user: user, socket: socket} do
    expired = Phoenix.Socket.assign(socket, :token_expires_at, System.system_time(:second) - 1)
    assert {:error, %{reason: "token_expired"}} = subscribe_and_join(expired, "user:#{user}", %{})
  end

  test "delivers broadcast_to_user with the versioned envelope", %{user: user, socket: socket} do
    {:ok, _, _socket} = subscribe_and_join(socket, "user:#{user}", %{})

    Core.Realtime.broadcast_to_user(user, "member.role.changed", %{role: "moderator"})

    assert_push "member.role.changed", %{
      type: "member.role.changed",
      version: 1,
      payload: %{role: "moderator"}
    }
  end

  test "does not deliver another user's events", %{user: user, socket: socket} do
    {:ok, _, _socket} = subscribe_and_join(socket, "user:#{user}", %{})

    Core.Realtime.broadcast_to_user(user_id(), "member.role.changed", %{})

    refute_push "member.role.changed", _
  end

  test "closes the topic when the token expires", %{user: user, socket: socket} do
    Process.flag(:trap_exit, true)
    soon = Phoenix.Socket.assign(socket, :token_expires_at, System.system_time(:second) + 2)
    {:ok, _, channel} = subscribe_and_join(soon, "user:#{user}", %{})
    ref = Process.monitor(channel.channel_pid)

    assert_push "token.expired", %{}, 4_000
    assert_receive {:DOWN, ^ref, :process, _, {:shutdown, :token_expired}}, 1_000
  end

  test "answers unknown events with an error", %{user: user, socket: socket} do
    {:ok, _, socket} = subscribe_and_join(socket, "user:#{user}", %{})
    ref = push(socket, "nope", %{})
    assert_reply ref, :error, %{type: "unknown-event"}
  end
end
