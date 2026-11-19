defmodule CoreWeb.EndpointTest do
  use ExUnit.Case, async: true

  # Las pruebas de canal llaman a connect/3 con connect_info armado a mano y
  # no pasan por el transporte; esta revisa la opción que sí usa el transporte.
  test "el socket lee el token de Sec-WebSocket-Protocol (auth_token del socket)" do
    assert [{"/socket", CoreWeb.UserSocket, opts}] = CoreWeb.Endpoint.__sockets__()
    assert opts[:auth_token] == true
  end
end
