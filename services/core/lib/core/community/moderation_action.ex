defmodule Core.Community.ModerationAction do
  use Ecto.Schema

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  schema "moderation_actions" do
    belongs_to :group, Core.Community.Group
    field :actor_id, :binary_id
    field :target_user_id, :binary_id
    field :action, Ecto.Enum, values: [:expel, :ban, :unban, :delete_message]
    field :target_ref, :string
    field :reason, :string
    field :created_at, :utc_datetime_usec, autogenerate: {DateTime, :utc_now, []}
  end
end
