defmodule CoreWeb.Presence do
  use Phoenix.Presence, otp_app: :core, pubsub_server: Core.PubSub
end
