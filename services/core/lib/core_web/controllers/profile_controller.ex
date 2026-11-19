defmodule CoreWeb.ProfileController do
  @moduledoc "Nombre visible del usuario autenticado (D-29)."
  use CoreWeb, :controller

  alias Core.Profiles

  action_fallback CoreWeb.FallbackController

  def show(conn, _params) do
    case Profiles.get_profile(conn.assigns.current_user_id) do
      nil -> {:error, :not_found}
      profile -> json(conn, %{data: profile_json(profile)})
    end
  end

  def update(conn, params) do
    attrs = Map.take(params, ["display_name"])

    with {:ok, profile} <- Profiles.put_display_name(conn.assigns.current_user_id, attrs) do
      json(conn, %{data: profile_json(profile)})
    end
  end

  defp profile_json(profile) do
    %{
      user_id: profile.user_id,
      display_name: profile.display_name,
      avatar_id: profile.avatar_id,
      updated_at: profile.updated_at
    }
  end
end
