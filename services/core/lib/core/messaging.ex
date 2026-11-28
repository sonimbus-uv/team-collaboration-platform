defmodule Core.Messaging do
  @moduledoc """
  MessagingService: envío idempotente de mensajes (CU-16) e historial
  paginado por canal (CU-20) en ScyllaDB (D-05).

  El acceso al canal (membresía y `min_role`, D-17) lo comprueba quien llama
  con `Core.Community.get_channel_for_member/2`; estas funciones reciben el
  canal ya autorizado.

  Diseño de tablas en `database/scylla/002_messages.cql` y
  `003_messages_by_client_id.cql`.
  """
  import Ecto.Changeset
  import Ecto.Query, only: [from: 2]

  require Logger

  alias Core.{Authorization, Community, Profiles, Realtime, Repo}
  alias Core.Community.{Channel, Group, Member, ModerationAction}
  alias Core.Messaging.TimeUUID
  alias Core.Scylla

  @max_content_length 4_000
  @default_page_size 50
  @max_page_size 100
  @preview_length 100

  @columns "channel_id, bucket, message_id, author_id, client_message_id, content, reply_to, file_id, edited_at, deleted, deleted_reason"

  @reserve_cql """
  INSERT INTO messages_by_client_id (author_id, client_message_id, channel_id, bucket, message_id)
  VALUES (?, ?, ?, ?, ?) IF NOT EXISTS
  """

  @insert_cql """
  INSERT INTO messages_by_channel
    (channel_id, bucket, message_id, author_id, client_message_id, content, reply_to, file_id, deleted)
  VALUES (?, ?, ?, ?, ?, ?, ?, ?, false)
  """

  @update_cql """
  UPDATE messages_by_channel SET content = ?, edited_at = ?
  WHERE channel_id = ? AND bucket = ? AND message_id = ?
  IF deleted = false
  """

  @delete_cql """
  UPDATE messages_by_channel SET deleted = true, deleted_reason = ?, deleted_by = ?
  WHERE channel_id = ? AND bucket = ? AND message_id = ?
  IF deleted = false
  """

  @get_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? AND message_id = ?"
  @page_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? LIMIT ?"
  @newer_after_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? AND message_id > ? ORDER BY message_id ASC LIMIT ?"
  @newer_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? ORDER BY message_id ASC LIMIT ?"
  @page_before_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? AND message_id < ? LIMIT ?"

  @doc """
  Guarda un mensaje de texto con idempotencia por `client_message_id`
  (CU-16 FA-02, RNF-CON-01).

  1. Reserva `(author_id, client_message_id)` con `INSERT ... IF NOT EXISTS`.
  2. Si la reserva es nueva, escribe el mensaje con el `message_id` que
     generó Core (D-08).
  3. Si ya existía, es un reintento: devuelve el mensaje guardado o, si se
     perdió entre la reserva y la escritura, lo escribe con los mismos ids.

  Devuelve `{:ok, message, :created}` o `{:ok, message, :duplicate}`; en el
  segundo caso no se debe volver a difundir.
  """
  def create_message(%Channel{} = channel, %Group{} = group, author_id, attrs) do
    with {:ok, message, outcome} <- store_message(channel, group, author_id, attrs) do
      {:ok, with_reply_preview(message, channel.id), outcome}
    end
  end

  defp store_message(%Channel{} = channel, %Group{} = group, author_id, attrs) do
    with :ok <- require_text(channel),
         :ok <- require_writable(group),
         {:ok, input} <- validate_message(attrs),
         :ok <- validate_reply_to(channel, input.reply_to) do
      message_id = TimeUUID.generate()
      bucket = bucket_of!(message_id)

      reserve = [
        author_id,
        input.client_message_id,
        channel.id,
        bucket,
        message_id
      ]

      case Scylla.execute(@reserve_cql, reserve) do
        {:ok, result} ->
          case Scylla.rows(result) do
            [%{"[applied]" => true}] ->
              write(channel.id, bucket, message_id, author_id, input, :created)

            [%{"[applied]" => false} = existing] ->
              retry(channel, existing, author_id, input)

            _ ->
              {:error, :unavailable}
          end

        error ->
          error
      end
    end
  end

  defp retry(channel, %{"channel_id" => channel_id} = existing, author_id, input) do
    if channel_id != channel.id do
      {:error, :client_message_id_conflict}
    else
      %{"bucket" => bucket, "message_id" => message_id} = existing

      case fetch(channel.id, bucket, message_id) do
        {:ok, nil} -> write(channel.id, bucket, message_id, author_id, input, :duplicate)
        {:ok, message} -> {:ok, message, :duplicate}
        error -> error
      end
    end
  end

  defp write(channel_id, bucket, message_id, author_id, input, outcome) do
    params = [
      channel_id,
      bucket,
      message_id,
      author_id,
      input.client_message_id,
      input.content,
      input.reply_to,
      input.file_id
    ]

    with {:ok, _} <- Scylla.execute(@insert_cql, params) do
      row = %{
        "channel_id" => channel_id,
        "message_id" => message_id,
        "author_id" => author_id,
        "client_message_id" => input.client_message_id,
        "content" => input.content,
        "reply_to" => input.reply_to,
        "file_id" => input.file_id,
        "edited_at" => nil,
        "deleted" => false
      }

      {:ok, to_message(row), outcome}
    end
  end

  @doc """
  Edita el contenido de un mensaje propio (CU-19). La condición `IF deleted =
  false` evita editar un mensaje que se eliminó entretanto.
  """
  def edit_message(%Channel{} = channel, %Group{} = group, user_id, message_id, attrs) do
    with :ok <- require_text(channel),
         :ok <- require_writable(group),
         {:ok, message} <- fetch_visible(channel, message_id),
         :ok <- require_author(message, user_id),
         {:ok, content} <- validate_edit(message, attrs) do
      edited_at = DateTime.utc_now()
      params = [content, edited_at, channel.id, bucket_of!(message.id), message.id]

      case conditional_update(@update_cql, params) do
        :applied ->
          {:ok,
           with_reply_preview(%{message | content: content, edited_at: edited_at}, channel.id)}

        :not_applied ->
          {:error, :not_found}

        error ->
          error
      end
    end
  end

  @doc """
  Elimina un mensaje (CU-19 FA-01, CU-15 FA-01). No borra la fila: marca
  `deleted` con su motivo (D-24).

  - El autor lo elimina con `deleted_reason = "author"`; no se admite en un
    grupo archivado.
  - Quien tenga el permiso `moderate` y un rol estrictamente superior al del
    autor (o el autor ya no sea miembro) lo elimina con `deleted_reason =
    "moderation"`, aunque el grupo esté archivado. Se registra la acción y se
    avisa al autor. No hay transacción entre ScyllaDB y PostgreSQL: si la
    auditoría falla, el borrado se mantiene y el fallo queda en el registro.
  """
  def delete_message(
        %Channel{} = channel,
        %Group{} = group,
        actor_id,
        actor_role,
        message_id,
        attrs \\ %{}
      ) do
    with :ok <- require_text(channel),
         {:ok, message} <- fetch_visible(channel, message_id) do
      if message.author_id == actor_id do
        with :ok <- require_writable(group) do
          mark_deleted(channel, message, "author", actor_id)
        end
      else
        moderate_delete(channel, group, actor_id, actor_role, message, attrs)
      end
    end
  end

  defp moderate_delete(channel, group, actor_id, actor_role, message, attrs) do
    with :ok <- Authorization.authorize(group.id, actor_role, :moderate),
         :ok <- require_higher_role(actor_role, author_role(group.id, message.author_id)),
         {:ok, reason} <- Community.cast_reason(attrs),
         {:ok, deleted} <- mark_deleted(channel, message, "moderation", actor_id) do
      audit_deletion(group, actor_id, message, reason)

      Realtime.broadcast_to_user(message.author_id, "moderation.applied", %{
        group_id: group.id,
        action: :delete_message,
        message_id: message.id,
        reason: reason
      })

      {:ok, deleted}
    end
  end

  defp audit_deletion(group, actor_id, message, reason) do
    case Repo.insert(%ModerationAction{
           group_id: group.id,
           actor_id: actor_id,
           target_user_id: message.author_id,
           action: :delete_message,
           target_ref: message.id,
           reason: reason
         }) do
      {:ok, _action} ->
        :ok

      {:error, _changeset} ->
        Logger.error("no se pudo registrar la eliminación moderada del mensaje #{message.id}")
    end
  end

  defp author_role(group_id, author_id) do
    Repo.one(
      from m in Member, where: m.group_id == ^group_id and m.user_id == ^author_id, select: m.role
    )
  end

  defp require_higher_role(_actor_role, nil), do: :ok

  defp require_higher_role(actor_role, author_role) do
    if actor_role != author_role and Member.role_at_least?(actor_role, author_role),
      do: :ok,
      else: {:error, :forbidden}
  end

  defp mark_deleted(channel, message, reason, deleted_by) do
    params = [reason, deleted_by, channel.id, bucket_of!(message.id), message.id]

    case conditional_update(@delete_cql, params) do
      :applied ->
        {:ok, %{message | deleted: true, content: nil, file_id: nil, deleted_reason: reason}}

      :not_applied ->
        {:error, :not_found}

      error ->
        error
    end
  end

  defp conditional_update(cql, params) do
    with {:ok, result} <- Scylla.execute(cql, params) do
      case Scylla.rows(result) do
        [%{"[applied]" => true}] -> :applied
        [%{"[applied]" => false}] -> :not_applied
        _ -> {:error, :unavailable}
      end
    end
  end

  defp fetch_visible(channel, message_id) do
    with {:ok, message_id} <- Ecto.UUID.cast(message_id),
         {:ok, _datetime} <- TimeUUID.to_datetime(message_id),
         {:ok, %{deleted: false} = message} <-
           fetch(channel.id, bucket_of!(message_id), message_id) do
      {:ok, message}
    else
      {:ok, _missing_or_deleted} -> {:error, :not_found}
      :error -> {:error, :not_found}
      error -> error
    end
  end

  defp require_author(%{author_id: author_id}, author_id), do: :ok
  defp require_author(_message, _user_id), do: {:error, :forbidden}

  defp validate_edit(message, attrs) do
    {%{}, %{content: :string}}
    |> cast(attrs, [:content], empty_values: [])
    |> validate_required([:content], message: "can't be blank")
    |> validate_length(:content, max: @max_content_length)
    |> validate_edit_minimum(message)
    |> apply_action(:update)
    |> case do
      {:ok, %{content: content}} -> {:ok, content}
      error -> error
    end
  end

  defp validate_edit_minimum(changeset, %{file_id: nil}),
    do: validate_length(changeset, :content, min: 1)

  defp validate_edit_minimum(changeset, _message), do: changeset

  @doc """
  Página del historial, del más reciente al más antiguo (CU-20).

  Opciones: `"before"` (cursor: el `id` del último mensaje entregado) y
  `"limit"` (1–100, 50 por defecto, D-22). Recorre los buckets mensuales hacia
  atrás hasta llenar la página o llegar al mes de creación del canal.
  `next_cursor` es `nil` cuando no quedan mensajes.
  """
  def list_messages(%Channel{} = channel, params) do
    with :ok <- require_text(channel),
         {:ok, page} <- validate_page(params),
         {:ok, messages} <- history(channel, page) do
      next_cursor = if length(messages) == page.limit, do: List.last(messages).id
      {:ok, %{messages: with_reply_previews(messages, channel.id), next_cursor: next_cursor}}
    end
  end

  defp history(channel, %{around: nil, before: before, limit: limit}) do
    start = if before, do: bucket_of!(before), else: TimeUUID.bucket(DateTime.utc_now())
    collect(channel.id, start, floor_bucket(channel), before, limit, [])
  end

  defp history(channel, %{around: around, limit: limit}) do
    bucket = bucket_of!(around)
    last_bucket = TimeUUID.bucket(DateTime.utc_now())

    with {:ok, target} <- fetch_around(channel, bucket, around),
         {:ok, newer} <- collect_newer(channel.id, bucket, last_bucket, around, div(limit, 2), []),
         older_limit = limit - 1 - length(newer),
         {:ok, older} <-
           collect(channel.id, bucket, floor_bucket(channel), around, older_limit, []) do
      {:ok, Enum.reverse(newer) ++ [target] ++ older}
    end
  end

  defp floor_bucket(channel), do: TimeUUID.bucket(channel.inserted_at)

  defp fetch_around(channel, bucket, around) do
    case fetch(channel.id, bucket, around) do
      {:ok, nil} -> {:error, :not_found}
      other -> other
    end
  end

  defp collect_newer(_channel_id, bucket, last_bucket, _after, _remaining, acc)
       when bucket > last_bucket,
       do: {:ok, acc}

  defp collect_newer(_channel_id, _bucket, _last_bucket, _after, 0, acc), do: {:ok, acc}

  defp collect_newer(channel_id, bucket, last_bucket, after_id, remaining, acc) do
    {cql, params} =
      if after_id,
        do: {@newer_after_cql, [channel_id, bucket, after_id, remaining]},
        else: {@newer_cql, [channel_id, bucket, remaining]}

    with {:ok, result} <- Scylla.execute(cql, params) do
      page = result |> Scylla.rows() |> Enum.map(&to_message/1)

      collect_newer(
        channel_id,
        TimeUUID.next_bucket(bucket),
        last_bucket,
        nil,
        remaining - length(page),
        acc ++ page
      )
    end
  end

  defp with_reply_previews(messages, channel_id) do
    originals =
      messages
      |> Enum.map(& &1.reply_to)
      |> Enum.reject(&is_nil/1)
      |> Enum.uniq()
      |> Map.new(&{&1, original(channel_id, &1)})

    names =
      originals
      |> Map.values()
      |> Enum.reject(&is_nil/1)
      |> Enum.map(& &1.author_id)
      |> Profiles.display_names()

    Enum.map(messages, fn message ->
      Map.put(message, :reply_preview, reply_preview(message.reply_to, originals, names))
    end)
  end

  defp with_reply_preview(message, channel_id),
    do: hd(with_reply_previews([message], channel_id))

  defp original(channel_id, message_id) do
    case fetch(channel_id, bucket_of!(message_id), message_id) do
      {:ok, %{deleted: false} = original} -> original
      _ -> nil
    end
  end

  defp reply_preview(nil, _originals, _names), do: nil

  defp reply_preview(reply_to, originals, names) do
    case Map.fetch!(originals, reply_to) do
      nil ->
        %{id: reply_to, unavailable: true}

      original ->
        %{
          id: reply_to,
          unavailable: false,
          author: %{id: original.author_id, display_name: Map.get(names, original.author_id)},
          content: String.slice(original.content || "", 0, @preview_length)
        }
    end
  end

  defp collect(_channel_id, bucket, floor, _before, _remaining, acc) when bucket < floor,
    do: {:ok, acc}

  defp collect(_channel_id, _bucket, _floor, _before, 0, acc), do: {:ok, acc}

  defp collect(channel_id, bucket, floor, before, remaining, acc) do
    {cql, params} =
      if before,
        do: {@page_before_cql, [channel_id, bucket, before, remaining]},
        else: {@page_cql, [channel_id, bucket, remaining]}

    with {:ok, result} <- Scylla.execute(cql, params) do
      page = result |> Scylla.rows() |> Enum.map(&to_message/1)

      collect(
        channel_id,
        TimeUUID.previous_bucket(bucket),
        floor,
        before,
        remaining - length(page),
        acc ++ page
      )
    end
  end

  defp fetch(channel_id, bucket, message_id) do
    params = [channel_id, bucket, message_id]

    with {:ok, result} <- Scylla.execute(@get_cql, params) do
      case Scylla.rows(result) do
        [row] -> {:ok, to_message(row)}
        [] -> {:ok, nil}
      end
    end
  end

  # Un mensaje eliminado no entrega content ni file_id (D-24).
  defp to_message(row) do
    deleted = row["deleted"] == true
    {:ok, created_at} = TimeUUID.to_datetime(row["message_id"])

    %{
      id: row["message_id"],
      channel_id: row["channel_id"],
      author_id: row["author_id"],
      client_message_id: row["client_message_id"],
      content: if(deleted, do: nil, else: row["content"]),
      reply_to: row["reply_to"],
      file_id: if(deleted, do: nil, else: row["file_id"]),
      created_at: created_at,
      edited_at: row["edited_at"],
      deleted: deleted,
      deleted_reason: if(deleted, do: row["deleted_reason"], else: nil)
    }
  end

  ## Validación

  defp validate_message(attrs) do
    types = %{
      client_message_id: Ecto.UUID,
      content: :string,
      reply_to: Ecto.UUID,
      file_id: Ecto.UUID
    }

    {%{}, types}
    |> cast(attrs, Map.keys(types), empty_values: [])
    |> validate_required([:client_message_id])
    |> validate_content()
    |> validate_change(:reply_to, &validate_timeuuid/2)
    |> apply_action(:insert)
    |> case do
      {:ok, input} ->
        {:ok,
         input
         |> Map.put_new(:reply_to, nil)
         |> Map.put_new(:file_id, nil)
         |> Map.update(:content, "", &(&1 || ""))}

      error ->
        error
    end
  end

  defp validate_content(changeset) do
    changeset = validate_length(changeset, :content, max: @max_content_length)

    if get_field(changeset, :file_id) do
      changeset
    else
      changeset
      |> validate_required([:content])
      |> validate_length(:content, min: 1)
    end
  end

  # La respuesta debe ser a un mensaje existente del mismo canal (CU-17).
  defp validate_reply_to(_channel, nil), do: :ok

  defp validate_reply_to(channel, reply_to) do
    case fetch(channel.id, bucket_of!(reply_to), reply_to) do
      {:ok, nil} -> {:error, invalid(:reply_to, "does not exist in this channel")}
      {:ok, _message} -> :ok
      error -> error
    end
  end

  defp validate_page(params) do
    types = %{before: Ecto.UUID, around: Ecto.UUID, limit: :integer}

    {%{}, types}
    |> cast(params, Map.keys(types))
    |> validate_number(:limit, greater_than_or_equal_to: 1, less_than_or_equal_to: @max_page_size)
    |> validate_change(:before, &validate_timeuuid/2)
    |> validate_change(:around, &validate_timeuuid/2)
    |> validate_exclusive(:before, :around)
    |> apply_action(:validate)
    |> case do
      {:ok, page} ->
        {:ok,
         %{
           before: Map.get(page, :before),
           around: Map.get(page, :around),
           limit: Map.get(page, :limit, @default_page_size)
         }}

      error ->
        error
    end
  end

  defp validate_exclusive(changeset, first, second) do
    if get_field(changeset, first) && get_field(changeset, second),
      do: add_error(changeset, second, "cannot be combined with #{first}"),
      else: changeset
  end

  defp validate_timeuuid(field, value) do
    case TimeUUID.to_datetime(value) do
      {:ok, _} -> []
      :error -> [{field, "must be a message id"}]
    end
  end

  defp invalid(field, message) do
    {%{}, %{field => :string}}
    |> change()
    |> add_error(field, message)
    |> Map.put(:action, :insert)
  end

  defp require_text(%Channel{type: :text}), do: :ok
  defp require_text(%Channel{}), do: {:error, :not_text_channel}

  defp require_writable(%Group{archived_at: nil}), do: :ok
  defp require_writable(%Group{}), do: {:error, :group_archived}

  defp bucket_of!(timeuuid) do
    {:ok, datetime} = TimeUUID.to_datetime(timeuuid)
    TimeUUID.bucket(datetime)
  end
end
