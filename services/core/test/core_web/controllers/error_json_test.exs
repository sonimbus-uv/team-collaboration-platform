defmodule CoreWeb.ErrorJSONTest do
  use CoreWeb.ConnCase, async: true

  test "renders 404" do
    assert CoreWeb.ErrorJSON.render("404.json", %{}) ==
             %{type: "about:blank", title: "Not Found", status: 404}
  end

  test "renders 500" do
    assert CoreWeb.ErrorJSON.render("500.json", %{}) ==
             %{type: "about:blank", title: "Internal Server Error", status: 500}
  end
end
