defmodule Core.AccessToken.HTTPFetcher do
  @moduledoc """
  Descarga el JWKS de Auth con `:httpc` (OTP), sin dependencias nuevas. Con
  HTTPS verifica la cadena y el nombre del servidor con los certificados del
  sistema. Respuesta de hasta 64 KiB, como el límite de Auth.
  """

  @max_body 65_536
  @timeout 2_000
  @connect_timeout 1_000

  def fetch(url) when is_binary(url) do
    request = {String.to_charlist(url), [{~c"accept", ~c"application/json"}]}

    http_options = [
      timeout: @timeout,
      connect_timeout: @connect_timeout,
      autoredirect: false,
      ssl: ssl_options(url)
    ]

    case :httpc.request(:get, request, http_options, body_format: :binary) do
      {:ok, {{_version, 200, _reason}, _headers, body}} when byte_size(body) <= @max_body ->
        {:ok, body}

      _ ->
        :error
    end
  end

  defp ssl_options("https://" <> _) do
    [
      verify: :verify_peer,
      cacerts: :public_key.cacerts_get(),
      depth: 4,
      customize_hostname_check: [match_fun: :public_key.pkix_verify_hostname_match_fun(:https)]
    ]
  end

  defp ssl_options(_url), do: []
end
