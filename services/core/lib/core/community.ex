defmodule Core.Community do
  @moduledoc """
  CommunityService: grupos, membresías, invitaciones y canales (CU-08, CU-10,
  CU-11, CU-13).

  Los grupos y canales borrados (`deleted_at`, D-19) se tratan como
  inexistentes. Quien no es miembro de un grupo recibe `:not_found`, para no
  revelar que el grupo existe.

  Los permisos por rol salen de `Core.Authorization` (D-18).
  """
  import Ecto.Query

  alias Core.Authorization
  alias Core.Repo
  alias Core.Community.{Ban, Channel, Group, Invitation, Member, ModerationAction}
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
             Repo.insert(%Member{group_id: group.id, user_id: user_id, role: :admin}),
           :ok <- Authorization.seed(group.id) do
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

  ## Roles y permisos

  def change_role(group_id, actor_id, target_id, role) do
    with {:ok, group, actor_role} <- get_group_for_member(group_id, actor_id),
         :ok <- Authorization.authorize(group.id, actor_role, :manage_roles),
         :ok <- require_writable(group),
         {:ok, new_role} <- cast_role(role),
         {:ok, target_id} <- cast_user_id(target_id),
         {:ok, change} <- apply_role_change(group.id, actor_id, target_id, new_role) do
      if change.previous_role != change.role do
        Core.Realtime.broadcast_to_user(target_id, "member.role.changed", %{
          group_id: group.id,
          role: change.role,
          previous_role: change.previous_role
        })
      end

      {:ok, change.member}
    end
  end

  defp apply_role_change(group_id, actor_id, target_id, new_role) do
    Repo.transact(fn ->
      admin_ids =
        Repo.all(
          from m in Member,
            where: m.group_id == ^group_id and m.role == :admin,
            order_by: m.user_id,
            select: m.user_id,
            lock: "FOR UPDATE"
        )

      target =
        Repo.one(
          from m in Member,
            where: m.group_id == ^group_id and m.user_id == ^target_id,
            lock: "FOR UPDATE"
        )

      cond do
        actor_id not in admin_ids ->
          {:error, :forbidden}

        is_nil(target) ->
          {:error, :not_found}

        target.role == :admin and new_role != :admin and admin_ids == [target_id] ->
          {:error, :last_admin}

        true ->
          if target.role != new_role do
            Repo.update_all(
              from(m in Member, where: m.group_id == ^group_id and m.user_id == ^target_id),
              set: [role: new_role]
            )
          end

          %{^target_id => display_name} = Core.Profiles.display_names([target_id])

          {:ok,
           %{
             previous_role: target.role,
             role: new_role,
             member: %{
               user_id: target_id,
               role: new_role,
               joined_at: target.joined_at,
               display_name: display_name
             }
           }}
      end
    end)
  end

  def list_roles(group_id, user_id) do
    with {:ok, group, _role} <- get_group_for_member(group_id, user_id) do
      {:ok, Enum.map(Member.roles(), &role_permissions(group.id, &1))}
    end
  end

  def update_role_permissions(group_id, actor_id, role, changes) do
    with {:ok, group, actor_role} <- get_group_for_member(group_id, actor_id),
         :ok <- Authorization.authorize(group.id, actor_role, :manage_roles),
         :ok <- require_writable(group),
         {:ok, role} <- cast_role(role),
         {:ok, changes} <- cast_permission_changes(changes) do
      rows =
        for {permission, enabled} <- changes do
          %{group_id: group.id, role: role, permission: permission, enabled: enabled}
        end

      {_count, _rows} =
        Repo.insert_all(Core.Authorization.RolePermission, rows,
          on_conflict: {:replace, [:enabled]},
          conflict_target: [:group_id, :role, :permission]
        )

      Authorization.invalidate_group(group.id)
      {:ok, role_permissions(group.id, role)}
    end
  end

  defp role_permissions(group_id, role) do
    enabled = Authorization.permissions(group_id, role)
    %{role: role, permissions: Map.new(Authorization.permissions(), &{&1, &1 in enabled})}
  end

  defp cast_role(role) when is_binary(role) do
    case Enum.find(Member.roles(), &(Atom.to_string(&1) == role)) do
      nil -> {:error, field_error(:role, "is invalid")}
      role -> {:ok, role}
    end
  end

  defp cast_role(_role), do: {:error, field_error(:role, "is invalid")}

  defp cast_user_id(user_id) do
    case Ecto.UUID.cast(user_id) do
      {:ok, user_id} -> {:ok, user_id}
      :error -> {:error, :not_found}
    end
  end

  defp cast_permission_changes(changes) when is_map(changes) and map_size(changes) > 0 do
    known = Map.new(Authorization.permissions(), &{Atom.to_string(&1), &1})

    errors =
      for {name, value} <- changes, reduce: [] do
        errors ->
          cond do
            not is_map_key(known, name) -> ["unknown permission #{inspect(name)}" | errors]
            known[name] in Authorization.admin_only() -> ["#{name} cannot be changed" | errors]
            not is_boolean(value) -> ["#{name} must be a boolean" | errors]
            true -> errors
          end
      end

    if errors == [] do
      {:ok, Map.new(changes, fn {name, value} -> {known[name], value} end)}
    else
      {:error, field_errors(:permissions, Enum.reverse(errors))}
    end
  end

  defp cast_permission_changes(_changes),
    do: {:error, field_error(:permissions, "must be a non-empty map")}

  defp field_error(field, message), do: field_errors(field, [message])

  defp field_errors(field, messages) do
    Enum.reduce(messages, Ecto.Changeset.change({%{}, %{field => :string}}), fn message, acc ->
      Ecto.Changeset.add_error(acc, field, message)
    end)
  end

  ## Moderación

  def expel_member(group_id, actor_id, target_id, attrs),
    do: moderate_member(:expel, group_id, actor_id, target_id, attrs)

  def ban_member(group_id, actor_id, target_id, attrs),
    do: moderate_member(:ban, group_id, actor_id, target_id, attrs)

  defp moderate_member(action, group_id, actor_id, target_id, attrs) do
    with {:ok, group, actor_role} <- get_group_for_member(group_id, actor_id),
         :ok <- Authorization.authorize(group.id, actor_role, :moderate),
         {:ok, target_id} <- cast_user_id(target_id),
         {:ok, reason} <- cast_reason(attrs),
         :ok <- reject_self(actor_id, target_id),
         {:ok, _action} <- apply_moderation(action, group.id, actor_id, target_id, reason) do
      Core.Realtime.disconnect_user_from_group(group.id, target_id)

      Core.Realtime.broadcast_to_user(target_id, "moderation.applied", %{
        group_id: group.id,
        action: action,
        reason: reason
      })

      :ok
    end
  end

  defp apply_moderation(action, group_id, actor_id, target_id, reason) do
    Repo.transact(fn ->
      rows =
        Repo.all(
          from m in Member,
            where: m.group_id == ^group_id and m.user_id in ^[actor_id, target_id],
            order_by: m.user_id,
            lock: "FOR UPDATE"
        )

      actor = Enum.find(rows, &(&1.user_id == actor_id))
      target = Enum.find(rows, &(&1.user_id == target_id))

      with %Member{} <- actor,
           :ok <- Authorization.authorize(group_id, actor.role, :moderate),
           %Member{} <- target || {:error, :not_found},
           :ok <- require_higher_role(actor.role, target.role) do
        Repo.delete_all(
          from m in Member, where: m.group_id == ^group_id and m.user_id == ^target_id
        )

        if action == :ban do
          Repo.insert!(%Ban{
            group_id: group_id,
            user_id: target_id,
            banned_by: actor_id,
            reason: reason
          })
        end

        record_action(action, group_id, actor_id, target_id, reason)
      else
        nil -> {:error, :not_found}
        {:error, reason} -> {:error, reason}
      end
    end)
  end

  def unban_member(group_id, actor_id, target_id) do
    with {:ok, group, actor_role} <- get_group_for_member(group_id, actor_id),
         :ok <- Authorization.authorize(group.id, actor_role, :moderate),
         {:ok, target_id} <- cast_user_id(target_id) do
      Repo.transact(fn ->
        case Repo.delete_all(
               from b in Ban, where: b.group_id == ^group.id and b.user_id == ^target_id
             ) do
          {1, _} -> record_action(:unban, group.id, actor_id, target_id, nil)
          {0, _} -> {:error, :not_found}
        end
      end)
      |> case do
        {:ok, _action} -> :ok
        error -> error
      end
    end
  end

  def list_bans(group_id, user_id) do
    with {:ok, group, role} <- get_group_for_member(group_id, user_id),
         :ok <- Authorization.authorize(group.id, role, :moderate) do
      bans =
        from(b in Ban,
          left_join: p in UserProfile,
          on: p.user_id == b.user_id,
          where: b.group_id == ^group.id,
          order_by: [desc: b.created_at, asc: b.user_id],
          select: %{
            user_id: b.user_id,
            banned_by: b.banned_by,
            reason: b.reason,
            created_at: b.created_at,
            display_name: p.display_name,
            account_status: p.account_status
          }
        )
        |> Repo.all()
        |> Enum.map(&anonymize/1)

      {:ok, bans}
    end
  end

  defp record_action(action, group_id, actor_id, target_id, reason) do
    Repo.insert(%ModerationAction{
      group_id: group_id,
      actor_id: actor_id,
      target_user_id: target_id,
      action: action,
      reason: reason
    })
  end

  defp reject_self(user_id, user_id), do: {:error, :forbidden}
  defp reject_self(_actor_id, _target_id), do: :ok

  defp require_higher_role(actor_role, target_role) do
    if actor_role != target_role and Member.role_at_least?(actor_role, target_role),
      do: :ok,
      else: {:error, :forbidden}
  end

  def cast_reason(attrs) do
    case attrs do
      %{"reason" => reason} when is_binary(reason) ->
        reason = String.trim(reason)

        cond do
          reason == "" -> {:ok, nil}
          String.length(reason) > 500 -> {:error, field_error(:reason, "is too long")}
          true -> {:ok, reason}
        end

      %{"reason" => nil} ->
        {:ok, nil}

      %{"reason" => _other} ->
        {:error, field_error(:reason, "is invalid")}

      _ ->
        {:ok, nil}
    end
  end

  ## Invitaciones

  @doc """
  Crea una invitación (CU-10). Requiere rol `moderator` o superior y un grupo
  vigente que no esté archivado.
  """
  def create_invitation(group_id, user_id, attrs) do
    with {:ok, group, role} <- get_group_for_member(group_id, user_id),
         :ok <- Authorization.authorize(group.id, role, :invite_members),
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
         :ok <- Authorization.authorize(group.id, role, :manage_channels),
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

  defp require_writable(%Group{archived_at: nil}), do: :ok
  defp require_writable(%Group{}), do: {:error, :group_archived}
end
