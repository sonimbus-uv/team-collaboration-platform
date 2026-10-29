defmodule Core.Community do
  @moduledoc """
  CommunityService: grupos, membresías, invitaciones y canales (CU-08, CU-10,
  CU-11, CU-13).

  Los grupos y canales borrados (`deleted_at`, D-19) se tratan como
  inexistentes. Quien no es miembro de un grupo recibe `:not_found`, para no
  revelar que el grupo existe.

  Mientras no exista `group_role_permissions` (D-18), los permisos usan los
  valores por defecto del catálogo: invitaciones para `moderator` y
  superiores, canales para `admin`.
  """
  import Ecto.Query

  alias Core.Repo
  alias Core.Community.{Ban, Channel, Group, Invitation, Member}
  alias Core.Profiles.UserProfile

  ## Grupos

  @doc """
  Crea un grupo y registra a su creador como `admin` en la misma transacción
  (CU-08 paso 5).
  """
  def create_group(user_id, attrs) do
    Repo.transact(fn ->
      with {:ok, group} <- %Group{} |> Group.create_changeset(attrs, user_id) |> Repo.insert(),
           {:ok, _member} <-
             Repo.insert(%Member{group_id: group.id, user_id: user_id, role: :admin}) do
        {:ok, group}
      end
    end)
  end

  @doc "Grupos vigentes del usuario, con su rol, por nombre."
  def list_groups_for_user(user_id) do
    from(g in Group,
      join: m in Member,
      on: m.group_id == g.id,
      where: m.user_id == ^user_id and is_nil(g.deleted_at),
      order_by: [asc: g.name, asc: g.id],
      select: {g, m.role}
    )
    |> Repo.all()
  end

  @doc "Grupo vigente y rol del usuario en él; `:not_found` si no es miembro."
  def get_group_for_member(group_id, user_id) do
    with {:ok, group_id} <- Ecto.UUID.cast(group_id),
         {%Group{} = group, role} <-
           from(g in Group,
             join: m in Member,
             on: m.group_id == g.id,
             where: g.id == ^group_id and m.user_id == ^user_id and is_nil(g.deleted_at),
             select: {g, m.role}
           )
           |> Repo.one() do
      {:ok, group, role}
    else
      _ -> {:error, :not_found}
    end
  end

  ## Miembros

  @doc """
  Miembros del grupo con su nombre visible, de mayor a menor rol y por fecha
  de ingreso. Solo para miembros del grupo.

  El nombre sale de `user_profiles` con `LEFT JOIN` (sin leer Auth, D-13):
  es `nil` si el usuario no tiene perfil o si su cuenta está eliminada
  (anonimizado).
  """
  def list_members(group_id, user_id) do
    with {:ok, group, _role} <- get_group_for_member(group_id, user_id) do
      members =
        from(m in Member,
          left_join: p in UserProfile,
          on: p.user_id == m.user_id,
          where: m.group_id == ^group.id,
          order_by: [desc: m.role, asc: m.joined_at, asc: m.user_id],
          select: %{
            user_id: m.user_id,
            role: m.role,
            joined_at: m.joined_at,
            display_name: p.display_name,
            account_status: p.account_status
          }
        )
        |> Repo.all()
        |> Enum.map(&anonymize/1)

      {:ok, members}
    end
  end

  defp anonymize(%{account_status: :deleted} = member),
    do: member |> Map.delete(:account_status) |> Map.put(:display_name, nil)

  defp anonymize(member), do: Map.delete(member, :account_status)

  ## Invitaciones

  @doc """
  Crea una invitación (CU-10). Requiere rol `moderator` o superior y un grupo
  vigente que no esté archivado.
  """
  def create_invitation(group_id, user_id, attrs) do
    with {:ok, group, role} <- get_group_for_member(group_id, user_id),
         :ok <- require_role(role, :moderator),
         :ok <- require_writable(group) do
      %Invitation{group_id: group.id, created_by: user_id}
      |> Invitation.create_changeset(attrs, DateTime.utc_now())
      |> Repo.insert()
    end
  end

  @doc """
  Une al usuario al grupo de la invitación `code` (CU-11).

  En una sola transacción: verifica la invitación y el bloqueo, inserta la
  membresía con `ON CONFLICT DO NOTHING` y descuenta el uso con un `UPDATE`
  condicionado. Si el `UPDATE` no afecta filas (otra petición agotó el cupo,
  la invitación caducó o se revocó entretanto), la transacción se revierte y
  la membresía no queda.

  Devuelve `{:ok, :joined, group, :member}` o
  `{:ok, :already_member, group, role}`; en el segundo caso no se descuenta
  ningún uso (EX-02).
  """
  def join_group(code, user_id) when is_binary(code) do
    Repo.transact(fn ->
      now = DateTime.utc_now()

      with {:ok, invitation, group} <- fetch_usable_invitation(code, now),
           :ok <- ensure_not_banned(group.id, user_id),
           {:ok, result} <- insert_membership(group.id, user_id) do
        case result do
          :already_member ->
            {:ok, {:already_member, group, member_role(group.id, user_id)}}

          :joined ->
            with :ok <- consume_invitation(invitation.id, now) do
              {:ok, {:joined, group, :member}}
            end
        end
      end
    end)
    |> case do
      {:ok, {result, group, role}} -> {:ok, result, group, role}
      {:error, reason} -> {:error, reason}
    end
  end

  def join_group(_code, _user_id), do: {:error, :invalid_invitation}

  defp fetch_usable_invitation(code, now) do
    from(i in Invitation,
      join: g in Group,
      on: g.id == i.group_id,
      where: i.code == ^code and is_nil(g.deleted_at) and is_nil(g.archived_at),
      select: {i, g}
    )
    |> Repo.one()
    |> case do
      {invitation, group} ->
        if Invitation.usable?(invitation, now),
          do: {:ok, invitation, group},
          else: {:error, :invalid_invitation}

      nil ->
        {:error, :invalid_invitation}
    end
  end

  defp ensure_not_banned(group_id, user_id) do
    if Repo.exists?(from b in Ban, where: b.group_id == ^group_id and b.user_id == ^user_id),
      do: {:error, :banned},
      else: :ok
  end

  # Una petición concurrente del mismo usuario espera aquí el bloqueo de la
  # clave primaria y, tras su commit, no inserta nada: queda como ya miembro.
  defp insert_membership(group_id, user_id) do
    row = %{group_id: group_id, user_id: user_id, role: :member, joined_at: DateTime.utc_now()}

    case Repo.insert_all(Member, [row],
           on_conflict: :nothing,
           conflict_target: [:group_id, :user_id]
         ) do
      {1, _} -> {:ok, :joined}
      {0, _} -> {:ok, :already_member}
    end
  end

  defp member_role(group_id, user_id) do
    Repo.one!(
      from m in Member, where: m.group_id == ^group_id and m.user_id == ^user_id, select: m.role
    )
  end

  # UPDATE ... WHERE uses < max_uses RETURNING: el cupo se decide en la base y
  # no en una lectura previa (CU-11 paso 6).
  defp consume_invitation(invitation_id, now) do
    {count, _} =
      from(i in Invitation,
        where:
          i.id == ^invitation_id and is_nil(i.revoked_at) and i.expires_at > ^now and
            (is_nil(i.max_uses) or i.uses < i.max_uses),
        select: i.id
      )
      |> Repo.update_all(inc: [uses: 1])

    if count == 1, do: :ok, else: {:error, :invalid_invitation}
  end

  ## Canales

  @doc "Canales vigentes del grupo que el rol del usuario puede ver (D-17)."
  def list_channels(group_id, user_id) do
    with {:ok, group, role} <- get_group_for_member(group_id, user_id) do
      visible_roles = Enum.filter(Member.roles(), &Member.role_at_least?(role, &1))

      channels =
        from(c in Channel,
          where:
            c.group_id == ^group.id and is_nil(c.deleted_at) and c.min_role in ^visible_roles,
          order_by: [asc: c.name]
        )
        |> Repo.all()

      {:ok, channels}
    end
  end

  @doc "Crea un canal (CU-13). Requiere rol `admin` y un grupo no archivado."
  def create_channel(group_id, user_id, attrs) do
    with {:ok, group, role} <- get_group_for_member(group_id, user_id),
         :ok <- require_role(role, :admin),
         :ok <- require_writable(group) do
      %Channel{group_id: group.id}
      |> Channel.create_changeset(attrs)
      |> Repo.insert()
    end
  end

  @doc """
  Canal vigente al que el usuario tiene acceso: es miembro del grupo vigente
  y su rol alcanza `min_role` (D-17). Devuelve también el grupo y el rol.
  """
  def get_channel_for_member(channel_id, user_id) do
    with {:ok, channel_id} <- Ecto.UUID.cast(channel_id),
         {%Channel{} = channel, %Group{} = group, role} <-
           from(c in Channel,
             join: g in Group,
             on: g.id == c.group_id,
             join: m in Member,
             on: m.group_id == g.id and m.user_id == ^user_id,
             where: c.id == ^channel_id and is_nil(c.deleted_at) and is_nil(g.deleted_at),
             select: {c, g, m.role}
           )
           |> Repo.one(),
         true <- Member.role_at_least?(role, channel.min_role) do
      {:ok, channel, group, role}
    else
      _ -> {:error, :not_found}
    end
  end

  ## Reglas comunes

  defp require_role(role, required) do
    if Member.role_at_least?(role, required), do: :ok, else: {:error, :forbidden}
  end

  defp require_writable(%Group{archived_at: nil}), do: :ok
  defp require_writable(%Group{}), do: {:error, :group_archived}
end
