defmodule CoreWeb.RoleController do
  use CoreWeb, :controller

  alias Core.Community

  action_fallback CoreWeb.FallbackController

  def index(conn, %{"group_id" => group_id}) do
    with {:ok, roles} <- Community.list_roles(group_id, conn.assigns.current_user_id) do
      json(conn, %{data: roles})
    end
  end

  def update(conn, %{"group_id" => group_id, "role" => role} = params) do
    with {:ok, permissions} <-
           Community.update_role_permissions(
             group_id,
             conn.assigns.current_user_id,
             role,
             params["permissions"]
           ) do
      json(conn, %{data: permissions})
    end
  end
end
