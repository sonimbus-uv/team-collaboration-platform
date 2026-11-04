defmodule CoreWeb.MessageJSON do
  @moduledoc """
  Representación JSON de mensajes (REST y WebSocket). `author` lleva el
  nombre visible al momento de la respuesta (`contracts/openapi/core.yaml`).
  """

  alias Core.Profiles

  @fields [
    :id,
    :channel_id,
    :author_id,
    :client_message_id,
    :content,
    :reply_to,
    :file_id,
    :created_at,
    :edited_at,
    :deleted
  ]

  @doc "Mensajes con su autor; una sola consulta a `user_profiles`."
  def messages(messages) do
    names = messages |> Enum.map(& &1.author_id) |> Profiles.display_names()
    Enum.map(messages, &message(&1, names))
  end

  def message(message), do: hd(messages([message]))

  defp message(message, names) do
    message
    |> Map.take(@fields)
    |> Map.put(:author, %{id: message.author_id, display_name: Map.get(names, message.author_id)})
  end
end
