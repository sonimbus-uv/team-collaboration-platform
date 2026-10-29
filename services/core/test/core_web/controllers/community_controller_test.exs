defmodule CoreWeb.CommunityControllerTest do
  use CoreWeb.ConnCase, async: true

  import Core.Fixtures

  setup %{conn: conn} do
    user = user_id()
    %{user: user, conn: authenticate(conn, user)}
  end

  test "sin usuario autenticado responde 401 con Problem Details" do
    conn = get(build_conn(), ~p"/api/v1/groups")

    assert %{"type" => "/problems/unauthorized", "status" => 401} = json_response(conn, 401)
    assert ["application/problem+json" <> _] = get_resp_header(conn, "content-type")
  end

  test "recorrido: crear grupo, invitación, unirse y crear canal", %{conn: conn, user: owner} do
    conn = post(conn, ~p"/api/v1/groups", %{"name" => "Equipo expo"})
    assert %{"id" => group_id, "role" => "admin"} = json_response(conn, 201)["data"]

    conn =
      post(authenticate(build_conn(), owner), ~p"/api/v1/groups/#{group_id}/invitations", %{
        "max_uses" => 3,
        "expires_in" => 3_600
      })

    assert %{"code" => code, "max_uses" => 3, "uses" => 0} = json_response(conn, 201)["data"]

    guest = user_id()

    conn =
      post(authenticate(build_conn(), guest), ~p"/api/v1/invitations/join", %{"code" => code})

    assert %{"id" => ^group_id, "role" => "member"} = json_response(conn, 201)["data"]

    conn =
      post(authenticate(build_conn(), guest), ~p"/api/v1/invitations/join", %{"code" => code})

    assert %{"id" => ^group_id} = json_response(conn, 200)["data"]

    conn =
      post(authenticate(build_conn(), owner), ~p"/api/v1/groups/#{group_id}/channels", %{
        "name" => "general",
        "type" => "text"
      })

    assert %{"name" => "general", "type" => "text"} = json_response(conn, 201)["data"]

    conn = get(authenticate(build_conn(), guest), ~p"/api/v1/groups/#{group_id}/channels")
    assert [%{"name" => "general"}] = json_response(conn, 200)["data"]

    conn = get(authenticate(build_conn(), guest), ~p"/api/v1/groups")
    assert [%{"id" => ^group_id, "role" => "member"}] = json_response(conn, 200)["data"]
  end

  test "datos inválidos devuelven 422 con errores por campo", %{conn: conn} do
    conn = post(conn, ~p"/api/v1/groups", %{"name" => ""})

    assert %{"type" => "/problems/validation-error", "errors" => %{"name" => [_]}} =
             json_response(conn, 422)
  end

  test "un grupo ajeno responde 404", %{conn: conn} do
    group = group_fixture(user_id())
    conn = get(conn, ~p"/api/v1/groups/#{group.id}")

    assert %{"type" => "/problems/not-found"} = json_response(conn, 404)
  end

  test "un miembro no puede crear canales (403)", %{conn: conn, user: user} do
    group = group_fixture(user_id())
    member_fixture(group, user, :member)

    conn = post(conn, ~p"/api/v1/groups/#{group.id}/channels", %{"name" => "x", "type" => "text"})
    assert %{"type" => "/problems/forbidden"} = json_response(conn, 403)
  end

  test "nombre de canal repetido devuelve 409", %{conn: conn, user: owner} do
    group = group_fixture(owner)
    channel_fixture(group, owner, %{"name" => "general"})

    conn =
      post(conn, ~p"/api/v1/groups/#{group.id}/channels", %{"name" => "general", "type" => "text"})

    assert %{"type" => "/problems/conflict", "errors" => %{"name" => _}} =
             json_response(conn, 409)
  end

  test "invitación no válida devuelve 422", %{conn: conn} do
    conn = post(conn, ~p"/api/v1/invitations/join", %{"code" => "no-existe"})
    assert %{"type" => "/problems/invitation-invalid"} = json_response(conn, 422)
  end
end
