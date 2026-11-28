defmodule Core.Repo.Migrations.CreateModerationActions do
  use Ecto.Migration

  def up do
    execute "CREATE TYPE moderation_action AS ENUM ('expel', 'ban', 'unban', 'delete_message')"

    create table(:moderation_actions, primary_key: false) do
      add :id, :binary_id, primary_key: true, default: fragment("gen_random_uuid()")
      add :group_id, references(:groups, type: :binary_id, on_delete: :nothing), null: false
      add :actor_id, :binary_id, null: false
      add :target_user_id, :binary_id, null: false
      add :action, :moderation_action, null: false
      add :target_ref, :text
      add :reason, :text
      add :created_at, :timestamptz, null: false, default: fragment("now()")
    end

    create constraint(:moderation_actions, :moderation_actions_reason_length,
             check: "reason IS NULL OR char_length(reason) <= 500"
           )

    create index(:moderation_actions, [:group_id, "created_at DESC"])

    execute """
    CREATE FUNCTION moderation_actions_append_only() RETURNS trigger AS $$
    BEGIN
      RAISE EXCEPTION 'moderation_actions is append-only';
    END;
    $$ LANGUAGE plpgsql
    """

    execute """
    CREATE TRIGGER moderation_actions_append_only
    BEFORE UPDATE OR DELETE ON moderation_actions
    FOR EACH ROW EXECUTE FUNCTION moderation_actions_append_only()
    """
  end

  def down do
    drop table(:moderation_actions)
    execute "DROP FUNCTION moderation_actions_append_only()"
    execute "DROP TYPE moderation_action"
  end
end
