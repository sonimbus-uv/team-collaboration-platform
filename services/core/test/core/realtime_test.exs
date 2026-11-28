defmodule Core.RealtimeTest do
  use CoreWeb.ChannelCase, async: true

  import Core.Fixtures
  import Core.AccessTokenHelpers

  alias Core.Realtime
  alias CoreWeb.UserSocket

  defp join_channel(user, channel) do
    {:ok, socket} = connect(UserSocket, %{}, connect_info: %{auth_token: token_for(user)})
    {:ok, _, socket} = subscribe_and_join(socket, "channel:#{channel.id}", %{})
    ref = Process.monitor(socket.channel_pid)
    {socket, ref}
  end

  setup do
    Process.flag(:trap_exit, true)
    owner = user_id()
    other = user_id()
    group = group_fixture(owner)
    other_group = group_fixture(owner, %{"name" => "Otro equipo"})
    general = channel_fixture(group, owner)
    random = channel_fixture(group, owner, %{"name" => "random"})
    elsewhere = channel_fixture(other_group, owner)

    member_fixture(group, other, :member)
    member_fixture(other_group, other, :member)

    %{
      owner: owner,
      other: other,
      group: group,
      general: general,
      random: random,
      elsewhere: elsewhere
    }
  end

  test "disconnect_user_from_group closes every channel of that user in the group", ctx do
    {_, general_ref} = join_channel(ctx.other, ctx.general)
    {_, random_ref} = join_channel(ctx.other, ctx.random)

    assert :ok = Realtime.disconnect_user_from_group(ctx.group.id, ctx.other)

    assert_receive {:DOWN, ^general_ref, :process, _, {:shutdown, :access_revoked}}
    assert_receive {:DOWN, ^random_ref, :process, _, {:shutdown, :access_revoked}}
  end

  test "disconnect_user_from_group leaves other users and other groups untouched", ctx do
    {_, owner_ref} = join_channel(ctx.owner, ctx.general)
    {_, elsewhere_ref} = join_channel(ctx.other, ctx.elsewhere)
    {_, target_ref} = join_channel(ctx.other, ctx.general)

    Realtime.disconnect_user_from_group(ctx.group.id, ctx.other)

    assert_receive {:DOWN, ^target_ref, :process, _, _}
    refute_receive {:DOWN, ^owner_ref, :process, _, _}
    refute_receive {:DOWN, ^elsewhere_ref, :process, _, _}
  end

  test "broadcast_to_group reaches the channels of the group only", ctx do
    {_, _} = join_channel(ctx.owner, ctx.general)
    {_, _} = join_channel(ctx.owner, ctx.elsewhere)

    Realtime.broadcast_to_group(ctx.group.id, "group.updated", %{name: "Nuevo"})

    assert_push "group.updated", %{type: "group.updated", version: 1, payload: %{name: "Nuevo"}}
    refute_push "group.updated", _
  end
end
