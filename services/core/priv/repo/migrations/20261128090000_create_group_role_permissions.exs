defmodule Core.Repo.Migrations.CreateGroupRolePermissions do
  use Ecto.Migration

  def up do
    create table(:group_role_permissions, primary_key: false) do
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), primary_key: true

      add :role, :group_role, primary_key: true
      add :permission, :text, primary_key: true
      add :enabled, :boolean, null: false, default: false
    end

    create constraint(:group_role_permissions, :group_role_permissions_catalog,
             check:
               "permission IN ('invite_members', 'moderate', 'manage_channels', 'manage_projects', 'manage_group', 'manage_roles')"
           )

    execute """
    INSERT INTO group_role_permissions (group_id, role, permission, enabled)
    SELECT g.id, d.role::group_role, d.permission, d.enabled
    FROM groups g
    CROSS JOIN (VALUES
      ('member', 'invite_members', false),
      ('member', 'moderate', false),
      ('member', 'manage_channels', false),
      ('member', 'manage_projects', false),
      ('member', 'manage_group', false),
      ('member', 'manage_roles', false),
      ('moderator', 'invite_members', true),
      ('moderator', 'moderate', true),
      ('moderator', 'manage_channels', false),
      ('moderator', 'manage_projects', false),
      ('moderator', 'manage_group', false),
      ('moderator', 'manage_roles', false),
      ('admin', 'invite_members', true),
      ('admin', 'moderate', true),
      ('admin', 'manage_channels', true),
      ('admin', 'manage_projects', true),
      ('admin', 'manage_group', true),
      ('admin', 'manage_roles', true)
    ) AS d(role, permission, enabled)
    ON CONFLICT DO NOTHING
    """
  end

  def down do
    drop table(:group_role_permissions)
  end
end
