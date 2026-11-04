defmodule CoreWeb.Plugs.AuthenticateTest do
  use CoreWeb.ConnCase, async: true

  import Core.AccessTokenHelpers

  test "token válido: pasa con el usuario de sub" do
    user = Ecto.UUID.generate()
    conn = build_conn() |> authenticate(user) |> get(~p"/api/v1/groups")

    assert json_response(conn, 200) == %{"data" => []}
  end

  test "sin encabezado, otro esquema o token inválido: 401" do
    for header <- [nil, "Basic abc", "Bearer ", "Bearer x.y.z"] do
      conn =
        if header, do: put_req_header(build_conn(), "authorization", header), else: build_conn()

      conn = get(conn, ~p"/api/v1/groups")

      assert %{"type" => "/problems/unauthorized"} = json_response(conn, 401)
      assert [~s(Bearer error="invalid_token")] = get_resp_header(conn, "www-authenticate")
    end
  end

  test "token vencido: 401" do
    user = Ecto.UUID.generate()
    now = System.system_time(:second)
    token = user |> valid_claims(now - 1_000) |> sign()

    conn =
      build_conn()
      |> put_req_header("authorization", "Bearer " <> token)
      |> get(~p"/api/v1/groups")

    assert json_response(conn, 401)
  end
end
