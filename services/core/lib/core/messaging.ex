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

  alias Core.Community.{Channel, Group}
  alias Core.Messaging.TimeUUID
  alias Core.Scylla

  @max_content_length 4_000
  @default_page_size 50
  @max_page_size 100

  @columns "channel_id, bucket, message_id, author_id, client_message_id, content, reply_to, file_id, edited_at, deleted"

  @reserve_cql """
  INSERT INTO messages_by_client_id (author_id, client_message_id, channel_id, bucket, message_id)
  VALUES (?, ?, ?, ?, ?) IF NOT EXISTS
  """

  @insert_cql """
  INSERT INTO messages_by_channel
    (channel_id, bucket, message_id, author_id, client_message_id, content, reply_to, deleted)
  VALUES (?, ?, ?, ?, ?, ?, ?, false)
  """

  @get_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? AND message_id = ?"
  @page_cql "SELECT #{@columns} FROM messages_by_channel WHERE channel_id = ? AND bucket = ? LIMIT ?"
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
      input.reply_to
    ]

    with {:ok, _} <- Scylla.execute(@insert_cql, params) do
      row = %{
        "channel_id" => channel_id,
        "message_id" => message_id,
        "author_id" => author_id,
        "client_message_id" => input.client_message_id,
        "content" => input.content,
        "reply_to" => input.reply_to,
        "file_id" => nil,
        "edited_at" => nil,
        "deleted" => false
      }

      {:ok, to_message(row), outcome}
    end
  end

  @doc """
  Página del historial, del más reciente al más antiguo (CU-20).

  Opciones: `"before"` (cursor: el `id` del último mensaje entregado) y
  `"limit"` (1–100, 50 por defecto, D-22). Recorre los buckets mensuales hacia
  atrás hasta llenar la página o llegar al mes de creación del canal.
  `next_cursor` es `nil` cuando no quedan mensajes.
  """
  def list_messages(%Channel{} = channel, params) do
    with :ok <- require_text(channel),
         {:ok, before, limit} <- validate_page(params) do
      start = if before, do: bucket_of!(before), else: TimeUUID.bucket(DateTime.utc_now())
      floor = TimeUUID.bucket(channel.inserted_at)

      with {:ok, messages} <- collect(channel.id, start, floor, before, limit, []) do
        next_cursor = if length(messages) == limit, do: List.last(messages).id
        {:ok, %{messages: messages, next_cursor: next_cursor}}
      end
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
      deleted: deleted
    }
  end

  ## Validación

  defp validate_message(attrs) do
    types = %{client_message_id: Ecto.UUID, content: :string, reply_to: Ecto.UUID}

    {%{}, types}
    |> cast(attrs, Map.keys(types), empty_values: [])
    |> validate_required([:client_message_id, :content])
    |> validate_length(:content, min: 1, max: @max_content_length)
    |> validate_change(:reply_to, &validate_timeuuid/2)
    |> apply_action(:insert)
    |> case do
      {:ok, input} -> {:ok, Map.put_new(input, :reply_to, nil)}
      error -> error
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
    types = %{before: Ecto.UUID, limit: :integer}

    {%{}, types}
    |> cast(params, Map.keys(types))
    |> validate_number(:limit, greater_than_or_equal_to: 1, less_than_or_equal_to: @max_page_size)
    |> validate_change(:before, &validate_timeuuid/2)
    |> apply_action(:validate)
    |> case do
      {:ok, page} -> {:ok, Map.get(page, :before), Map.get(page, :limit, @default_page_size)}
      error -> error
    end
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
