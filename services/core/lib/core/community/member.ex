defmodule Core.Community.Member do
  @moduledoc """
  Membresía de un usuario en un grupo. La clave primaria `(group_id, user_id)`
  impide membresías duplicadas (CU-11 EX-02). `user_id` es `auth.accounts.id`,
  sin FK (D-13).
  """
  use Ecto.Schema

  @primary_key false
  @foreign_key_type :binary_id

  @roles [:member, :moderator, :admin]

  schema "group_members" do
    belongs_to :group, Core.Community.Group, primary_key: true
    field :user_id, :binary_id, primary_key: true
    field :role, Ecto.Enum, values: @roles, default: :member
    field :joined_at, :utc_datetime_usec, autogenerate: {DateTime, :utc_now, []}
  end

  @doc "Roles en orden creciente, igual que el tipo `group_role` (D-18)."
  def roles, do: @roles

  @doc "Indica si `role` es igual o superior a `required`."
  def role_at_least?(role, required) do
    rank(role) >= rank(required)
  end

  defp rank(role), do: Enum.find_index(@roles, &(&1 == role))
end
