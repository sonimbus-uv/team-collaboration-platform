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

  def update(conn, %{"group_id" => group_id, "user_id" => user_id} = params) do
    with {:ok, member} <-
           Community.change_role(group_id, conn.assigns.current_user_id, user_id, params["role"]) do
      json(conn, %{data: member})
    end
  end

  def leave(conn, %{"group_id" => group_id}) do
    with :ok <- Community.leave_group(group_id, conn.assigns.current_user_id) do
      send_resp(conn, :no_content, "")
    end
  end
end
