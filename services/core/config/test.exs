import Config

# Configure your database
#
# The MIX_TEST_PARTITION environment variable can be used
# to provide built-in test partitioning in CI environment.
# Run `mix help test` for more information.
# Las pruebas usan la misma base y el mismo esquema `core` que desarrollo:
# `core_service` no puede crear bases de datos (D-13). El sandbox revierte
# cada prueba. Credenciales en config/runtime.exs.
config :core, Core.Repo,
  pool: Ecto.Adapters.SQL.Sandbox,
  pool_size: System.schedulers_online() * 2

# We don't run a server during test. If one is required,
# you can enable the server option below.
config :core, CoreWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  secret_key_base: "iT17mWOMncdH3uuguEpO9CNJQP1j0BEKlzhEZQghU2FY+ppVe47FNN3lTRFyotOY",
  server: false

# Print only warnings and errors during test
config :logger, level: :warning

# Initialize plugs at runtime for faster test compilation
config :phoenix, :plug_init_mode, :runtime

# Sort query params output of verified routes for robust url comparisons
config :phoenix,
  sort_verified_routes_query_params: true
