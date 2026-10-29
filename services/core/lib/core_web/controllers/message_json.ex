defmodule CoreWeb.MessageJSON do
  @moduledoc "Representación JSON de mensajes (REST y WebSocket)."

  def message(message) do
    Map.take(message, [
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
    ])
  end
end
