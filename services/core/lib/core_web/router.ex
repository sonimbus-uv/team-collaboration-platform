defmodule CoreWeb.Router do
  use CoreWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", CoreWeb do
    pipe_through :api

    get "/health", HealthController, :show
  end

  pipeline :authenticated do
    plug CoreWeb.Plugs.Authenticate
  end

  scope "/api/v1", CoreWeb do
    pipe_through [:api, :authenticated]

    resources "/groups", GroupController, only: [:index, :create, :show] do
      resources "/channels", ChannelController, only: [:index, :create]
      resources "/members", MemberController, only: [:index]
      resources "/invitations", InvitationController, only: [:create]
    end

    get "/channels/:channel_id/messages", MessageController, :index

    get "/me/profile", ProfileController, :show
    put "/me/profile", ProfileController, :update

    # El código va en el cuerpo y no en la ruta, para que no quede en los
    # registros de acceso.
    post "/invitations/join", InvitationController, :join
  end
end
