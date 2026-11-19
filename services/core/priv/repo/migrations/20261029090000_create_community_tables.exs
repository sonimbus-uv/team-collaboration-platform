defmodule Core.Repo.Migrations.CreateCommunityTables do
  @moduledoc """
  Tablas del recorrido mínimo: grupos, membresías, canales, invitaciones,
  bloqueos y copia local de perfiles (CU-08, CU-11, CU-13, CU-16, CU-20).

  Diseño en docs/database/README.md (esquema `core`) y
  docs/database/core-erd-input.md. Las columnas de usuario son
  `auth.accounts.id` sin FK (D-13). Las FK internas no usan ON DELETE CASCADE:
  grupos y canales se borran de forma lógica (D-19).
  """
  use Ecto.Migration

  def up do
    # El orden de los valores permite comparar roles: member < moderator < admin (D-18).
    execute "CREATE TYPE group_role AS ENUM ('member', 'moderator', 'admin')"
    execute "CREATE TYPE channel_type AS ENUM ('text', 'voice')"
    execute "CREATE TYPE account_status AS ENUM ('active', 'suspended', 'deleted')"

    create table(:groups, primary_key: false) do
      add :id, :binary_id, primary_key: true, default: fragment("gen_random_uuid()")
      add :name, :text, null: false
      add :description, :text
      add :image_id, :binary_id
      add :created_by, :binary_id, null: false
      add :archived_at, :timestamptz
      add :deleted_at, :timestamptz
      timestamps(type: :timestamptz, default: fragment("now()"))
    end

    create constraint(:groups, :groups_name_length, check: "char_length(name) BETWEEN 2 AND 100")

    create constraint(:groups, :groups_description_length,
             check: "description IS NULL OR char_length(description) <= 500"
           )

    create table(:group_members, primary_key: false) do
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), primary_key: true

      add :user_id, :binary_id, primary_key: true
      add :role, :group_role, null: false, default: "member"
      add :joined_at, :timestamptz, null: false, default: fragment("now()")
    end

    create index(:group_members, [:user_id])

    create table(:channels, primary_key: false) do
      add :id, :binary_id, primary_key: true, default: fragment("gen_random_uuid()")
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), null: false
      add :name, :text, null: false
      add :description, :text
      add :type, :channel_type, null: false
      add :min_role, :group_role, null: false, default: "member"
      add :deleted_at, :timestamptz
      timestamps(type: :timestamptz, default: fragment("now()"))
    end

    create constraint(:channels, :channels_name_format,
             check: "char_length(name) BETWEEN 1 AND 80 AND name ~ '^[a-z0-9-]+$'"
           )

    create constraint(:channels, :channels_description_length,
             check: "description IS NULL OR char_length(description) <= 500"
           )

    create unique_index(:channels, [:group_id, :name],
             where: "deleted_at IS NULL",
             name: :channels_group_id_name_active_index
           )

    create table(:invitations, primary_key: false) do
      add :id, :binary_id, primary_key: true, default: fragment("gen_random_uuid()")
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), null: false
      add :code, :text, null: false
      add :created_by, :binary_id, null: false

      add :expires_at, :timestamptz,
        null: false,
        default: fragment("now() + interval '7 days'")

      add :max_uses, :integer
      add :uses, :integer, null: false, default: 0
      add :revoked_at, :timestamptz
      add :created_at, :timestamptz, null: false, default: fragment("now()")
    end

    create unique_index(:invitations, [:code])

    create constraint(:invitations, :invitations_max_uses_range,
             check: "max_uses IS NULL OR max_uses BETWEEN 1 AND 100"
           )

    create constraint(:invitations, :invitations_uses_range,
             check: "uses >= 0 AND (max_uses IS NULL OR uses <= max_uses)"
           )

    create constraint(:invitations, :invitations_expiry_range,
             check:
               "expires_at >= created_at + interval '1 hour' AND expires_at <= created_at + interval '30 days'"
           )

    create table(:group_bans, primary_key: false) do
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), primary_key: true

      add :user_id, :binary_id, primary_key: true
      add :banned_by, :binary_id, null: false
      add :reason, :text
      add :created_at, :timestamptz, null: false, default: fragment("now()")
    end

    # Copia local del perfil. Para la expo la llena Core con el nombre que envía
    # el cliente (D-29); version 0 hace que el primer user.updated de Auth gane (D-20).
    create table(:user_profiles, primary_key: false) do
      add :user_id, :binary_id, primary_key: true
      add :display_name, :text, null: false
      add :avatar_id, :text
      add :account_status, :account_status, null: false, default: "active"
      add :version, :bigint, null: false, default: 0
      add :updated_at, :timestamptz, null: false, default: fragment("now()")
    end

    create constraint(:user_profiles, :user_profiles_display_name_length,
             check: "char_length(display_name) BETWEEN 1 AND 100"
           )
  end

  def down do
    drop table(:user_profiles)
    drop table(:group_bans)
    drop table(:invitations)
    drop table(:channels)
    drop table(:group_members)
    drop table(:groups)

    execute "DROP TYPE account_status"
    execute "DROP TYPE channel_type"
    execute "DROP TYPE group_role"
  end
end
