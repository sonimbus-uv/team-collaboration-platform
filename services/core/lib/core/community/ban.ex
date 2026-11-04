defmodule Core.Community.Ban do
  @moduledoc """
  Bloqueo de un usuario en un grupo: impide su reingreso (CU-15 FA-03).
  """
  use Ecto.Schema

  @primary_key false
  @foreign_key_type :binary_id

  schema "group_bans" do
    belongs_to :group, Core.Community.Group, primary_key: true
    field :user_id, :binary_id, primary_key: true
    field :banned_by, :binary_id
    field :reason, :string
    field :created_at, :utc_datetime_usec, autogenerate: {DateTime, :utc_now, []}
  end
end
