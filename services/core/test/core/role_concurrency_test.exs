defmodule Core.RoleConcurrencyTest do
  use ExUnit.Case, async: false

  import Ecto.Query
  import Core.Fixtures

  alias Core.{Community, Repo}
  alias Ecto.Adapters.SQL.Sandbox

  @moduletag :concurrency

  setup do
    Sandbox.mode(Repo, :auto)

    first = user_id()
    second = user_id()
    group = group_fixture(first)
    member_fixture(group, second, :admin)

    on_exit(fn ->
      Sandbox.mode(Repo, :auto)
      group_id = Ecto.UUID.dump!(group.id)
      Repo.query!("DELETE FROM group_role_permissions WHERE group_id = $1", [group_id])
      Repo.query!("DELETE FROM group_members WHERE group_id = $1", [group_id])
      Repo.query!("DELETE FROM groups WHERE id = $1", [group_id])
      Sandbox.mode(Repo, :manual)
    end)

    %{first: first, second: second, group: group}
  end

  defp admins(group) do
    Repo.all(
      from m in Community.Member,
        where: m.group_id == ^group.id and m.role == :admin,
        select: m.user_id
    )
  end

  defp race(fun_a, fun_b) do
    tasks = [Task.async(fun_a), Task.async(fun_b)]
    Task.await_many(tasks, 10_000)
  end

  test "two admins demoting each other leave exactly one admin", ctx do
    results =
      race(
        fn -> Community.change_role(ctx.group.id, ctx.first, ctx.second, "member") end,
        fn -> Community.change_role(ctx.group.id, ctx.second, ctx.first, "member") end
      )

    assert [{:error, _reason}, {:ok, _member}] = Enum.sort_by(results, &elem(&1, 0))
    assert length(admins(ctx.group)) == 1
  end

  test "two admins stepping down at once cannot empty the group", ctx do
    results =
      race(
        fn -> Community.change_role(ctx.group.id, ctx.first, ctx.first, "member") end,
        fn -> Community.change_role(ctx.group.id, ctx.second, ctx.second, "member") end
      )

    assert [{:error, :last_admin}, {:ok, _member}] = Enum.sort_by(results, &elem(&1, 0))
    assert length(admins(ctx.group)) == 1
  end
end
