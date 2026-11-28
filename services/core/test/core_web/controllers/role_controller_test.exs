defmodule CoreWeb.RoleControllerTest do
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

  test "PATCH changes the role and returns the member", ctx do
    conn =
      patch(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}", %{
        "role" => "moderator"
      })

    member = ctx.member
    assert %{"data" => %{"user_id" => ^member, "role" => "moderator"}} = json_response(conn, 200)
  end

  test "PATCH by a non admin is 403", ctx do
    conn =
      patch(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.owner}", %{
        "role" => "member"
      })

    assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
  end

  test "PATCH on a stranger is 404 and an invalid role is 422", ctx do
    conn =
      patch(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{user_id()}", %{
        "role" => "moderator"
      })

    assert json_response(conn, 404)

    conn =
      patch(
        build_conn() |> authenticate(ctx.owner),
        ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}",
        %{
          "role" => "owner"
        }
      )

    assert %{"type" => "/problems/validation-error", "errors" => %{"role" => ["is invalid"]}} =
             json_response(conn, 422)
  end

  test "PATCH that would leave no admin is 409 last-admin", ctx do
    conn =
      patch(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.owner}", %{
        "role" => "member"
      })

    assert %{"type" => "/problems/last-admin", "status" => 409} = json_response(conn, 409)
  end

  test "PATCH on an archived group is 409 group-archived", ctx do
    ctx.group |> Ecto.Changeset.change(archived_at: DateTime.utc_now()) |> Core.Repo.update!()

    conn =
      patch(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/members/#{ctx.member}", %{
        "role" => "moderator"
      })

    assert %{"type" => "/problems/group-archived"} = json_response(conn, 409)
  end

  test "GET roles lists the three roles for any member", ctx do
    conn = get(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/roles")

    assert %{
             "data" => [
               %{"role" => "member", "permissions" => %{"moderate" => false}},
               %{"role" => "moderator", "permissions" => %{"moderate" => true}},
               %{"role" => "admin", "permissions" => %{"manage_roles" => true}}
             ]
           } = json_response(conn, 200)
  end

  test "GET roles for a stranger is 404", ctx do
    conn = get(authenticate(build_conn(), user_id()), ~p"/api/v1/groups/#{ctx.group.id}/roles")
    assert json_response(conn, 404)
  end

  test "PUT permissions updates a role", ctx do
    conn =
      put(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/roles/moderator/permissions", %{
        "permissions" => %{"manage_channels" => true, "moderate" => false}
      })

    assert %{
             "data" => %{
               "role" => "moderator",
               "permissions" => %{"manage_channels" => true, "moderate" => false}
             }
           } = json_response(conn, 200)
  end

  test "PUT permissions rejects admin-only permissions with 422 and non admins with 403", ctx do
    conn =
      put(ctx.admin_conn, ~p"/api/v1/groups/#{ctx.group.id}/roles/moderator/permissions", %{
        "permissions" => %{"manage_roles" => true}
      })

    assert %{"type" => "/problems/validation-error", "errors" => %{"permissions" => [_]}} =
             json_response(conn, 422)

    conn =
      put(ctx.member_conn, ~p"/api/v1/groups/#{ctx.group.id}/roles/member/permissions", %{
        "permissions" => %{"moderate" => true}
      })

    assert json_response(conn, 403)
  end
end
