defmodule Core.Profiles.UserProfile do
  @moduledoc """
  Copia local del perfil de un usuario (D-06, D-20). `user_id` es
  `auth.accounts.id`, sin FK (D-13).

  Para la expo, el nombre visible lo declara el usuario (D-29) y `version`
  queda en 0; el primer `user.updated` de Auth trae una versión mayor y gana.
  """
  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:user_id, :binary_id, autogenerate: false}

  schema "user_profiles" do
    field :display_name, :string
    field :avatar_id, :string
    field :account_status, Ecto.Enum, values: [:active, :suspended, :deleted], default: :active
    field :version, :integer, default: 0
    field :updated_at, :utc_datetime_usec
  end

  @doc "Nombre visible de 1–100 caracteres, sin espacios en los extremos."
  def display_name_changeset(profile, attrs) do
    profile
    |> cast(attrs, [:display_name])
    |> update_change(:display_name, &String.trim/1)
    |> validate_required([:display_name])
    |> validate_length(:display_name, min: 1, max: 100)
    |> check_constraint(:display_name, name: :user_profiles_display_name_length)
  end
end
