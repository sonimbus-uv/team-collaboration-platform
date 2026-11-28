defmodule CoreWeb.ModerationController do
  use CoreWeb, :controller

  alias Core.Community

  action_fallback CoreWeb.FallbackController

  def expel(conn, %{"group_id" => group_id, "user_id" => user_id} = params) do
    with :ok <-
           Community.expel_member(group_id, conn.assigns.current_user_id, user_id, params) do
      send_resp(conn, :no_content, "")
    end
  end

  def ban(conn, %{"group_id" => group_id, "user_id" => user_id} = params) do
    with :ok <- Community.ban_member(group_id, conn.assigns.current_user_id, user_id, params) do
      send_resp(conn, :no_content, "")
    end
  end

  def index(conn, %{"group_id" => group_id}) do
    with {:ok, bans} <- Community.list_bans(group_id, conn.assigns.current_user_id) do
      json(conn, %{data: bans})
    end
  end

  def delete(conn, %{"group_id" => group_id, "user_id" => user_id}) do
    with :ok <- Community.unban_member(group_id, conn.assigns.current_user_id, user_id) do
      send_resp(conn, :no_content, "")
    end
  end
end
