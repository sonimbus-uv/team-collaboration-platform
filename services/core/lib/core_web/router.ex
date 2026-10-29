defmodule CoreWeb.Router do
  use CoreWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/", CoreWeb do
    pipe_through :api

    get "/health", HealthController, :show
  end

  scope "/api/v1", CoreWeb do
    pipe_through :api
  end
end
