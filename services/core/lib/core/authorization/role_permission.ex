defmodule Core.Authorization.RolePermission do
  use Ecto.Schema

  @primary_key false
  @foreign_key_type :binary_id

  @permissions [
    :invite_members,
    :moderate,
    :manage_channels,
    :manage_projects,
    :manage_group,
    :manage_roles
  ]

  schema "group_role_permissions" do
    belongs_to :group, Core.Community.Group, primary_key: true
    field :role, Ecto.Enum, values: Core.Community.Member.roles(), primary_key: true
    field :permission, Ecto.Enum, values: @permissions, primary_key: true
    field :enabled, :boolean, default: false
  end

  def permissions, do: @permissions
end
