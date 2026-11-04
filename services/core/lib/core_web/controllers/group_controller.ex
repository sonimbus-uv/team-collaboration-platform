defmodule CoreWeb.GroupController do
  @moduledoc "Grupos (CU-08)."
  use CoreWeb, :controller

  alias Core.Community
  alias CoreWeb.CommunityJSON

  action_fallback CoreWeb.FallbackController

  def index(conn, _params) do
    groups = Community.list_groups_for_user(conn.assigns.current_user_id)

    json(conn, %{data: Enum.map(groups, fn {group, role} -> CommunityJSON.group(group, role) end)})
  end

  def create(conn, params) do
    with {:ok, group} <- Community.create_group(conn.assigns.current_user_id, params) do
      conn
      |> put_status(:created)
      |> put_resp_header("location", ~p"/api/v1/groups/#{group.id}")
      |> json(%{data: CommunityJSON.group(group, :admin)})
    end
  end

  def show(conn, %{"id" => id}) do
    with {:ok, group, role} <- Community.get_group_for_member(id, conn.assigns.current_user_id) do
      json(conn, %{data: CommunityJSON.group(group, role)})
    end
  end
end
