defmodule CoreWeb.ProfileControllerTest do
  use CoreWeb.ConnCase, async: true

  setup %{conn: conn} do
    user = Ecto.UUID.generate()
    %{user: user, conn: authenticate(conn, user)}
  end

  test "PUT guarda el nombre y GET lo devuelve", %{conn: conn, user: user} do
    conn = put(conn, ~p"/api/v1/me/profile", %{"display_name" => "Ana"})
    assert %{"user_id" => ^user, "display_name" => "Ana"} = json_response(conn, 200)["data"]

    conn = get(authenticate(build_conn(), user), ~p"/api/v1/me/profile")
    assert %{"display_name" => "Ana"} = json_response(conn, 200)["data"]
  end

  test "GET sin perfil responde 404", %{conn: conn} do
    assert %{"type" => "/problems/not-found"} =
             conn |> get(~p"/api/v1/me/profile") |> json_response(404)
  end

  test "PUT con nombre vacío responde 422", %{conn: conn} do
    conn = put(conn, ~p"/api/v1/me/profile", %{"display_name" => ""})
    assert %{"errors" => %{"display_name" => [_]}} = json_response(conn, 422)
  end
end
