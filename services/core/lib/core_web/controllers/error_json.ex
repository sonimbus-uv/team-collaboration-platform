defmodule CoreWeb.ErrorJSON do
  @moduledoc """
  Errores que Phoenix genera fuera de los controladores (ruta inexistente,
  JSON mal formado, excepciones), con la forma de Problem Details. Nunca
  incluyen detalles internos (RNF-SEG-04).

  Ver config/config.exs.
  """

  def render(template, _assigns) do
    status = template |> String.split(".") |> hd() |> String.to_integer()

    %{
      type: "about:blank",
      title: Phoenix.Controller.status_message_from_template(template),
      status: status
    }
  end
end
