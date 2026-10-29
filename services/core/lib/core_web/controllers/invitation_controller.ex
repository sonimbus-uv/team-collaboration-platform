defmodule CoreWeb.InvitationController do
  @moduledoc "Invitaciones (CU-10) y unión a grupos (CU-11)."
  use CoreWeb, :controller

  alias Core.Community
  alias CoreWeb.CommunityJSON

  action_fallback CoreWeb.FallbackController

  def create(conn, %{"group_id" => group_id} = params) do
    attrs = Map.take(params, ["expires_in", "max_uses"])

    with {:ok, invitation} <-
           Community.create_invitation(group_id, conn.assigns.current_user_id, attrs) do
      conn
      |> put_status(:created)
      |> json(%{data: CommunityJSON.invitation(invitation)})
    end
  end

  # 201 si el usuario se une; 200 si ya era miembro (CU-11 EX-02).
  def join(conn, %{"code" => code}) do
    with {:ok, result, group, role} <- Community.join_group(code, conn.assigns.current_user_id) do
      conn
      |> put_status(if result == :joined, do: :created, else: :ok)
      |> json(%{data: CommunityJSON.group(group, role)})
    end
  end
end
