defmodule Core.Community.Channel do
  @moduledoc """
  Canal de texto o de voz de un grupo (CU-13). El nombre es único por grupo
  entre canales no borrados (índice parcial, D-19) y `min_role` define quién
  lo ve y lo usa (D-17).
  """
  use Ecto.Schema
  import Ecto.Changeset

  alias Core.Community.Member

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id
  @timestamps_opts [type: :utc_datetime_usec]

  schema "channels" do
    belongs_to :group, Core.Community.Group
    field :name, :string
    field :description, :string
    field :type, Ecto.Enum, values: [:text, :voice]
    field :min_role, Ecto.Enum, values: Member.roles(), default: :member
    field :deleted_at, :utc_datetime_usec

    timestamps()
  end

  @doc "Nombre de 1–80 caracteres: minúsculas, dígitos y guiones (D-22)."
  def create_changeset(channel, attrs) do
    channel
    |> cast(attrs, [:name, :description, :type, :min_role])
    |> update_change(:name, &String.trim/1)
    |> validate_required([:name, :type])
    |> validate_length(:name, min: 1, max: 80)
    |> validate_format(:name, ~r/^[a-z0-9-]+$/,
      message: "must contain only lowercase letters, digits and hyphens"
    )
    |> validate_length(:description, max: 500)
    |> check_constraint(:name, name: :channels_name_format)
    |> check_constraint(:description, name: :channels_description_length)
    |> unique_constraint(:name,
      name: :channels_group_id_name_active_index,
      message: "has already been taken"
    )
  end
end
