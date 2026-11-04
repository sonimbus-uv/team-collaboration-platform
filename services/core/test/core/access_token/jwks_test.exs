defmodule Core.AccessToken.JWKSTest do
  use ExUnit.Case, async: true

  # Los casos de fallo registran una advertencia esperada.
  @moduletag :capture_log

  import Core.AccessTokenHelpers, only: [jwks_json: 1]

  alias Core.AccessToken.JWKS

  setup do
    {public, _} = :crypto.generate_key(:eddsa, :ed25519)

    {:ok, agent} =
      Agent.start_link(fn -> %{calls: 0, response: {:ok, jwks_json([{"k1", public}])}} end)

    name = :"jwks_#{System.unique_integer([:positive])}"
    %{agent: agent, name: name, public: public}
  end

  def fetch(agent) do
    Agent.get_and_update(agent, fn state ->
      {state.response, %{state | calls: state.calls + 1}}
    end)
  end

  defp start(name, agent, opts \\ []) do
    opts = Keyword.merge([name: name, fetcher: {__MODULE__, :fetch, [agent]}], opts)
    start_supervised!({JWKS, opts})
  end

  defp calls(agent), do: Agent.get(agent, & &1.calls)

  test "descarga una vez y sirve desde la caché", %{agent: agent, name: name, public: public} do
    start(name, agent)

    assert {:ok, ^public} = JWKS.get_key(name, "k1")
    assert {:ok, ^public} = JWKS.get_key(name, "k1")
    assert calls(agent) == 1
  end

  test "kid desconocido: una descarga acotada por intervalo", %{agent: agent, name: name} do
    start(name, agent, min_refresh_ms: 60_000)

    assert {:ok, _} = JWKS.get_key(name, "k1")
    assert {:error, :unknown_kid} = JWKS.get_key(name, "atacante-1")
    assert {:error, :unknown_kid} = JWKS.get_key(name, "atacante-2")
    assert calls(agent) == 1
  end

  test "vencido y sin poder descargar: no usa claves viejas", %{agent: agent, name: name} do
    start(name, agent, max_age_ms: 0, min_refresh_ms: 0)

    Agent.update(agent, &%{&1 | response: :error})
    assert {:error, :unavailable} = JWKS.get_key(name, "k1")
  end

  test "sin JWKS inicial: unavailable", %{agent: agent, name: name} do
    Agent.update(agent, &%{&1 | response: {:error, :econnrefused}})
    start(name, agent)

    assert {:error, :unavailable} = JWKS.get_key(name, "k1")
  end

  describe "parse/1" do
    test "acepta claves Ed25519 públicas", %{public: public} do
      assert {:ok, %{"k1" => ^public}} = JWKS.parse(jwks_json([{"k1", public}]))
    end

    test "ignora claves con d, otra curva u otro uso", %{public: public} do
      x = Base.url_encode64(public, padding: false)

      body =
        Jason.encode!(%{
          keys: [
            %{kty: "OKP", crv: "Ed25519", kid: "privada", x: x, d: x},
            %{kty: "OKP", crv: "X25519", kid: "x25519", x: x},
            %{kty: "OKP", crv: "Ed25519", kid: "enc", use: "enc", x: x},
            %{kty: "OKP", crv: "Ed25519", kid: "corta", x: "AAAA"},
            %{kty: "OKP", crv: "Ed25519", kid: "buena", x: x}
          ]
        })

      assert {:ok, keys} = JWKS.parse(body)
      assert Map.keys(keys) == ["buena"]
    end

    test "rechaza kid repetido, lista vacía o JSON inválido", %{public: public} do
      assert :error = JWKS.parse(jwks_json([{"k1", public}, {"k1", public}]))
      assert :error = JWKS.parse(~s({"keys":[]}))
      assert :error = JWKS.parse("no json")
    end
  end
end
