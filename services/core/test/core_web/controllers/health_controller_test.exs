defmodule CoreWeb.HealthControllerTest do
  use CoreWeb.ConnCase, async: true

  test "GET /health responde ok cuando PostgreSQL está disponible", %{conn: conn} do
    conn = get(conn, ~p"/health")

    assert json_response(conn, 200) == %{"status" => "ok"}
  end
end
