defmodule Core.Community.Invitation do
  @moduledoc """
  Invitación a un grupo (CU-10, CU-11). Es válida si no está revocada, no ha
  caducado y le quedan usos (`max_uses` nulo = ilimitado). Caducidad de 1 h a
  30 días, 7 días por defecto; de 1 a 100 usos (D-22).
  """
  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @min_expires_in 3_600
  @max_expires_in 30 * 86_400
  @default_expires_in 7 * 86_400

  schema "invitations" do
    belongs_to :group, Core.Community.Group
    field :code, :string
    field :created_by, :binary_id
    field :expires_at, :utc_datetime_usec
    field :max_uses, :integer
    field :uses, :integer, default: 0
    field :revoked_at, :utc_datetime_usec
    field :created_at, :utc_datetime_usec

    # Duración solicitada en segundos; no se guarda.
    field :expires_in, :integer, virtual: true
  end

  @doc """
  `created_at` y `expires_at` salen del mismo instante para que el `CHECK` de
  la caducidad (1 h – 30 días) se cumpla de forma exacta.
  """
  def create_changeset(invitation, attrs, now) do
    invitation
    |> cast(attrs, [:expires_in, :max_uses])
    |> validate_number(:expires_in,
      greater_than_or_equal_to: @min_expires_in,
      less_than_or_equal_to: @max_expires_in
    )
    |> validate_number(:max_uses, greater_than_or_equal_to: 1, less_than_or_equal_to: 100)
    |> put_expiry(now)
    |> put_change(:code, generate_code())
    |> check_constraint(:max_uses, name: :invitations_max_uses_range)
    |> check_constraint(:expires_in, name: :invitations_expiry_range)
    |> unique_constraint(:code)
  end

  @doc "Indica si la invitación se puede usar en el instante `now`."
  def usable?(%__MODULE__{} = invitation, now) do
    is_nil(invitation.revoked_at) and DateTime.compare(invitation.expires_at, now) == :gt and
      (is_nil(invitation.max_uses) or invitation.uses < invitation.max_uses)
  end

  defp put_expiry(changeset, now) do
    expires_in = get_field(changeset, :expires_in) || @default_expires_in

    changeset
    |> put_change(:created_at, now)
    |> put_change(:expires_at, DateTime.add(now, expires_in, :second))
  end

  # 96 bits aleatorios: 16 caracteres base64url, sin relleno.
  defp generate_code do
    :crypto.strong_rand_bytes(12) |> Base.url_encode64(padding: false)
  end
end
