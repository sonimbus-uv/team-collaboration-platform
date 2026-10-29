defmodule CoreWeb.MemberController do
  @moduledoc "Miembros de un grupo con su nombre visible."
  use CoreWeb, :controller

  alias Core.Community

  action_fallback CoreWeb.FallbackController

  def index(conn, %{"group_id" => group_id}) do
    with {:ok, members} <- Community.list_members(group_id, conn.assigns.current_user_id) do
      json(conn, %{data: members})
    end
  end
end
