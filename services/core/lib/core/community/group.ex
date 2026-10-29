defmodule Core.Community.Group do
  @moduledoc """
  Grupo de trabajo (CU-08). Se borra de forma lógica con `deleted_at` (D-19) y
  `archived_at` lo deja en solo lectura (CU-09 FA-01).
  """
  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id
  @timestamps_opts [type: :utc_datetime_usec]

  schema "groups" do
    field :name, :string
    field :description, :string
    field :image_id, :binary_id
    field :created_by, :binary_id
    field :archived_at, :utc_datetime_usec
    field :deleted_at, :utc_datetime_usec

    timestamps()
  end

  @doc "Datos que captura el usuario al crear el grupo (D-22)."
  def create_changeset(group, attrs, created_by) do
    group
    |> cast(attrs, [:name, :description, :image_id])
    |> update_change(:name, &String.trim/1)
    |> validate_required([:name])
    |> validate_length(:name, min: 2, max: 100)
    |> validate_length(:description, max: 500)
    |> put_change(:created_by, created_by)
    |> check_constraint(:name, name: :groups_name_length)
    |> check_constraint(:description, name: :groups_description_length)
  end
end
