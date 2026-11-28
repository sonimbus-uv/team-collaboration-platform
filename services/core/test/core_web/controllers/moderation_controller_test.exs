defmodule CoreWeb.ModerationControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures

  setup %{conn: conn} do
    owner = user_id()
    group = group_fixture(owner)
    member = user_id()
    member_fixture(group, member, :member)

    %{
      owner: owner,
      member: member,
      group: group,
      admin_conn: authenticate(conn, owner),
      member_conn: authenticate(build_conn(), member)
    }
  end

  test "POST expel answers 204 and the member is gone", ctx do
    conn =
      post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}/expel", %{
        "reason" => "Spam"
      })

    assert response(conn, 204)

    conn = get(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members")
    assert [%{"role" => "admin"}] = json_response(conn, 200)["data"]
  end

  test "POST ban answers 204, lists the ban and DELETE lifts it", ctx do
    conn = post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}/ban", %{})
    assert response(conn, 204)

    conn = get(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/bans")
    member = ctx.member
    assert [%{"user_id" => ^member, "reason" => nil}] = json_response(conn, 200)["data"]

    conn = delete(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/bans/#{ctx.member}")
    assert response(conn, 204)

    conn = get(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/bans")
    assert [] = json_response(conn, 200)["data"]
  end

  test "a member cannot moderate (403) and a lower role cannot touch an admin", ctx do
    conn =
      post(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.owner}/expel", %{})

    assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)

    conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/bans")
    assert json_response(conn, 403)
  end

  test "an unknown target is 404 and a long reason is 422", ctx do
    conn =
      post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{user_id()}/expel", %{})

    assert json_response(conn, 404)

    conn =
      post(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}/ban", %{
        "reason" => String.duplicate("x", 501)
      })

    assert %{"type" => "/problems/validation-error", "errors" => %{"reason" => [_]}} =
             json_response(conn, 422)
  end

  test "unbanning someone not banned is 404", ctx do
    conn = delete(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/bans/#{ctx.member}")
    assert json_response(conn, 404)
  end
end
