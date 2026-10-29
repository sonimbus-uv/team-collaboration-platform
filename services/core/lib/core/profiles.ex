defmodule Core.Profiles do
  @moduledoc """
  Copia local de perfiles (D-06, D-20) y nombre visible declarado por el
  usuario para la expo (D-29).
  """
  import Ecto.Query

  alias Core.Repo
  alias Core.Profiles.UserProfile

  @doc """
  Guarda el nombre visible que envía el cliente (D-29).

  Es un *upsert* condicionado a `version = 0`: si Auth ya publicó el perfil
  (`version > 0`, D-20), el nombre lo administra Auth y se devuelve
  `{:error, :managed_by_auth}` sin cambiar nada.
  """
  def put_display_name(user_id, attrs) do
    changeset = UserProfile.display_name_changeset(%UserProfile{user_id: user_id}, attrs)

    with {:ok, profile} <- Ecto.Changeset.apply_action(changeset, :update) do
      now = DateTime.utc_now()

      on_conflict =
        from(p in UserProfile,
          where: p.version == 0,
          update: [set: [display_name: ^profile.display_name, updated_at: ^now]]
        )

      row = %{user_id: user_id, display_name: profile.display_name, updated_at: now}

      case Repo.insert_all(UserProfile, [row],
             on_conflict: on_conflict,
             conflict_target: :user_id,
             returning: true
           ) do
        {1, [profile]} -> {:ok, profile}
        {0, []} -> {:error, :managed_by_auth}
      end
    end
  end

  @doc "Perfil local del usuario o `nil`."
  def get_profile(user_id), do: Repo.get(UserProfile, user_id)
end
