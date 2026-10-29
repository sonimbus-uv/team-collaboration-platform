import Config

# config/runtime.exs is executed for all environments, including
# during releases. It is executed after compilation and before the
# system starts, so it is typically used to load production configuration
# and secrets from environment variables or elsewhere. Do not define
# any compile-time configuration in here, as it won't be applied.
# The block below contains prod specific runtime configuration.

# ## Using releases
#
# If you use `mix release`, you need to explicitly enable the server
# by passing the PHX_SERVER=true when you start it:
#
#     PHX_SERVER=true bin/core start
#
# Alternatively, you can use `mix phx.gen.release` to generate a `bin/server`
# script that automatically sets the env var above.
if System.get_env("PHX_SERVER") do
  config :core, CoreWeb.Endpoint, server: true
end

# Core usa la serie 3000 de los servicios internos (Auth en 3001).
config :core, CoreWeb.Endpoint, http: [port: String.to_integer(System.get_env("PORT", "3002"))]

if config_env() in [:dev, :test] do
  # Usuario y esquema propios de Core (D-13, database/postgres/). La contraseña
  # es CORE_DB_PASSWORD de infrastructure/.env; ver services/core/.env.example.
  config :core, Core.Repo,
    username: System.get_env("CORE_DB_USER", "core_service"),
    password:
      System.get_env("CORE_DB_PASSWORD") ||
        raise("falta la variable de entorno CORE_DB_PASSWORD; ver services/core/.env.example"),
    hostname: System.get_env("CORE_DB_HOST", "localhost"),
    port: String.to_integer(System.get_env("CORE_DB_PORT", "5432")),
    database: System.get_env("CORE_DB_NAME", "team_collaboration")
end

# Orígenes permitidos en el WebSocket (/socket), separados por comas, con el
# formato de check_origin de Phoenix ("https://app.example", "//localhost").
# Los clientes nativos (JavaFX y Flutter) no envían Origin y siempre pasan;
# la lista solo filtra navegadores. Sin la variable: en desarrollo, localhost
# con cualquier esquema y puerto; en producción, el host de PHX_HOST.
if config_env() in [:dev, :prod] do
  ws_origins =
    case {System.get_env("CORE_WS_ALLOWED_ORIGINS"), config_env()} do
      {nil, :dev} -> ["//localhost", "//127.0.0.1"]
      {nil, :prod} -> true
      {value, _env} -> value |> String.split(",", trim: true) |> Enum.map(&String.trim/1)
    end

  config :core, CoreWeb.Endpoint, check_origin: ws_origins
end

# ScyllaDB (D-05): nodos como "host:puerto", separados por comas.
config :core, Core.Scylla,
  nodes: System.get_env("CORE_SCYLLA_NODES", "localhost:9042") |> String.split(",", trim: true),
  keyspace: System.get_env("CORE_SCYLLA_KEYSPACE", "sonimbus_core")

# Verificación del access JWT de Auth (contracts/token-profile.md). El issuer
# debe ser igual a AUTH_JWT_ISSUER de Auth; las pruebas usan config/test.exs.
if config_env() in [:dev, :prod] do
  {default_issuer, default_jwks_url} =
    if config_env() == :dev,
      do: {"https://auth.example.invalid", "http://localhost:3001/.well-known/jwks.json"},
      else: {nil, nil}

  jwt_issuer =
    System.get_env("CORE_JWT_ISSUER", default_issuer) ||
      raise "falta la variable de entorno CORE_JWT_ISSUER"

  jwks_url =
    System.get_env("CORE_JWKS_URL", default_jwks_url) ||
      raise "falta la variable de entorno CORE_JWKS_URL"

  config :core, Core.AccessToken,
    issuer: jwt_issuer,
    jwks_fetcher: {Core.AccessToken.HTTPFetcher, :fetch, [jwks_url]}
end

if config_env() == :prod do
  database_url =
    System.get_env("DATABASE_URL") ||
      raise """
      environment variable DATABASE_URL is missing.
      For example: ecto://USER:PASS@HOST/DATABASE
      """

  maybe_ipv6 = if System.get_env("ECTO_IPV6") in ~w(true 1), do: [:inet6], else: []

  config :core, Core.Repo,
    # ssl: true,
    url: database_url,
    pool_size: String.to_integer(System.get_env("POOL_SIZE") || "10"),
    # For machines with several cores, consider starting multiple pools of `pool_size`
    # pool_count: 4,
    socket_options: maybe_ipv6

  # The secret key base is used to sign/encrypt cookies and other secrets.
  # A default value is used in config/dev.exs and config/test.exs but you
  # want to use a different value for prod and you most likely don't want
  # to check this value into version control, so we use an environment
  # variable instead.
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      raise """
      environment variable SECRET_KEY_BASE is missing.
      You can generate one by calling: mix phx.gen.secret
      """

  host = System.get_env("PHX_HOST") || "example.com"

  config :core, :dns_cluster_query, System.get_env("DNS_CLUSTER_QUERY")

  config :core, CoreWeb.Endpoint,
    url: [host: host, port: 443, scheme: "https"],
    http: [
      # Enable IPv6 and bind on all interfaces.
      # Set it to  {0, 0, 0, 0, 0, 0, 0, 1} for local network only access.
      # See the documentation on https://bandit.hexdocs.pm/Bandit.html#t:options/0
      # for details about using IPv6 vs IPv4 and loopback vs public addresses.
      ip: {0, 0, 0, 0, 0, 0, 0, 0}
    ],
    secret_key_base: secret_key_base

  # ## SSL Support
  #
  # To get SSL working, you will need to add the `https` key
  # to your endpoint configuration:
  #
  #     config :core, CoreWeb.Endpoint,
  #       https: [
  #         ...,
  #         port: 443,
  #         cipher_suite: :strong,
  #         keyfile: System.get_env("SOME_APP_SSL_KEY_PATH"),
  #         certfile: System.get_env("SOME_APP_SSL_CERT_PATH")
  #       ]
  #
  # The `cipher_suite` is set to `:strong` to support only the
  # latest and more secure SSL ciphers. This means old browsers
  # and clients may not be supported. You can set it to
  # `:compatible` for wider support.
  #
  # `:keyfile` and `:certfile` expect an absolute path to the key
  # and cert in disk or a relative path inside priv, for example
  # "priv/ssl/server.key". For all supported SSL configuration
  # options, see https://plug.hexdocs.pm/Plug.SSL.html#configure/1
  #
  # We also recommend setting `force_ssl` in your config/prod.exs,
  # ensuring no data is ever sent via http, always redirecting to https:
  #
  #     config :core, CoreWeb.Endpoint,
  #       force_ssl: [hsts: true]
  #
  # Check `Plug.SSL` for all available options in `force_ssl`.
end
