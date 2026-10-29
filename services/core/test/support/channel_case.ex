defmodule CoreWeb.ChannelCase do
  @moduledoc """
  Pruebas de canales de Phoenix con el sandbox de Ecto.
  """
  use ExUnit.CaseTemplate

  using do
    quote do
      import Phoenix.ChannelTest
      import CoreWeb.ChannelCase

      @endpoint CoreWeb.Endpoint
    end
  end

  setup tags do
    Core.DataCase.setup_sandbox(tags)
    :ok
  end
end
