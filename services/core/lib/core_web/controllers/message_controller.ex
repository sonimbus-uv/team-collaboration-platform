defmodule CoreWeb.MessageController do
  @moduledoc "Historial paginado de un canal de texto (CU-20)."
  use CoreWeb, :controller

  alias Core.{Community, Messaging}
  alias CoreWeb.MessageJSON

  action_fallback CoreWeb.FallbackController

  def index(conn, %{"channel_id" => channel_id} = params) do
    with {:ok, channel, _group, _role} <-
           Community.get_channel_for_member(channel_id, conn.assigns.current_user_id),
         {:ok, page} <- Messaging.list_messages(channel, Map.take(params, ["before", "limit"])) do
      json(conn, %{
        data: MessageJSON.messages(page.messages),
        next_cursor: page.next_cursor
      })
    end
  end
end
