defmodule CoreWeb.VoiceController do
  use CoreWeb, :controller

  alias Core.Voice

  action_fallback CoreWeb.FallbackController

  def create(conn, %{"channel_id" => channel_id}) do
    with {:ok, issued} <- Voice.issue_token(channel_id, conn.assigns.current_user_id) do
      conn
      |> put_resp_header("cache-control", "no-store")
      |> json(%{
        data: %{
          token: issued.token,
          url: issued.url,
          expires_at: DateTime.to_iso8601(issued.expires_at)
        }
      })
    end
  end
end
