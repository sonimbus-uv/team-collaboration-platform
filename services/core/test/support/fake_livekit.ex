defmodule Core.FakeLiveKit do
  @moduledoc false
  @behaviour Plug

  import Plug.Conn

  def start(test_pid, status \\ 200) do
    {:ok, server} =
      ExUnit.Callbacks.start_supervised(
        {Bandit,
         plug: {__MODULE__, {test_pid, status}}, port: 0, ip: :loopback, startup_log: false}
      )

    {:ok, {_ip, port}} = ThousandIsland.listener_info(server)
    "ws://127.0.0.1:#{port}"
  end

  @impl true
  def init(opts), do: opts

  @impl true
  def call(conn, {test_pid, status}) do
    {:ok, body, conn} = read_body(conn)
    [authorization] = get_req_header(conn, "authorization")

    send(test_pid, {:livekit_request, conn.method, conn.request_path, authorization, body})

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(status, "{}")
  end
end
