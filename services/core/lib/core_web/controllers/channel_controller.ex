defmodule CoreWeb.ChannelController do
  @moduledoc "Canales de un grupo (CU-13)."
  use CoreWeb, :controller

  alias Core.Community
  alias CoreWeb.CommunityJSON

  action_fallback CoreWeb.FallbackController

  def index(conn, %{"group_id" => group_id}) do
    with {:ok, channels} <- Community.list_channels(group_id, conn.assigns.current_user_id) do
      json(conn, %{data: Enum.map(channels, &CommunityJSON.channel/1)})
    end
  end

  def create(conn, %{"group_id" => group_id} = params) do
    attrs = Map.take(params, ["name", "description", "type", "min_role"])

    with {:ok, channel} <- Community.create_channel(group_id, conn.assigns.current_user_id, attrs) do
      conn
      |> put_status(:created)
      |> json(%{data: CommunityJSON.channel(channel)})
    end
  end

  def update(conn, %{"group_id" => group_id, "id" => id} = params) do
    attrs = Map.take(params, ["name", "description"])

    with {:ok, channel} <-
           Community.update_channel(group_id, conn.assigns.current_user_id, id, attrs) do
      json(conn, %{data: CommunityJSON.channel(channel)})
    end
  end

  def delete(conn, %{"group_id" => group_id, "id" => id}) do
    with :ok <- Community.delete_channel(group_id, conn.assigns.current_user_id, id) do
      send_resp(conn, :no_content, "")
    end
  end
end
