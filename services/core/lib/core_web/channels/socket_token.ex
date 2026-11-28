defmodule CoreWeb.SocketToken do
  def time_left(socket) do
    ms_left = socket.assigns.token_expires_at * 1_000 - System.system_time(:millisecond)
    if ms_left > 0, do: {:ok, ms_left}, else: {:error, :token_expired}
  end

  def expire_in(ms_left) do
    Process.send_after(self(), :token_expired, ms_left)
    :ok
  end
end
