defmodule Core.AccessTokenHelpers do
  @moduledoc """
  Clave Ed25519 de prueba, JWKS correspondiente y firma de access JWT con el
  perfil de `contracts/token-profile.md`.
  """

  @kid "test-key-1"

  @doc """
  Genera el par de claves de la ejecución. Se llama una sola vez desde
  `test/test_helper.exs`, antes de las pruebas: generarlo a demanda dejaba
  que dos pruebas asíncronas crearan claves distintas a la vez.
  """
  def init_keypair! do
    :persistent_term.put({__MODULE__, :keypair}, :crypto.generate_key(:eddsa, :ed25519))
  end

  @doc "Par de claves `{pública, privada}` de la ejecución."
  def keypair, do: :persistent_term.get({__MODULE__, :keypair})

  def kid, do: @kid

  @doc "Fetcher de `Core.AccessToken.JWKS` en pruebas."
  def fetch_jwks do
    {public, _private} = keypair()
    {:ok, jwks_json([{@kid, public}])}
  end

  def jwks_json(keys) do
    Jason.encode!(%{
      keys:
        for {kid, public} <- keys do
          %{
            kty: "OKP",
            crv: "Ed25519",
            use: "sig",
            alg: "EdDSA",
            kid: kid,
            x: Base.url_encode64(public, padding: false)
          }
        end
    })
  end

  def valid_claims(user_id, now \\ System.system_time(:second)) do
    %{
      "iss" => "https://auth.test.invalid",
      "aud" => ["sonimbus-core", "sonimbus-files", "sonimbus-auth"],
      "sub" => user_id,
      "sid" => Ecto.UUID.generate(),
      "iat" => now,
      "exp" => now + 900,
      "jti" => Ecto.UUID.generate()
    }
  end

  def default_header, do: %{"alg" => "EdDSA", "typ" => "at+jwt", "kid" => @kid}

  @doc "Firma claims y encabezado ya codificables en JSON."
  def sign(claims, header \\ default_header()) do
    sign_raw(Jason.encode!(header), Jason.encode!(claims))
  end

  @doc "Firma JSON literal; permite probar claves repetidas."
  def sign_raw(header_json, claims_json, private \\ elem(keypair(), 1)) do
    input = b64(header_json) <> "." <> b64(claims_json)
    signature = :crypto.sign(:eddsa, :none, input, [private, :ed25519])
    input <> "." <> b64(signature)
  end

  def token_for(user_id), do: user_id |> valid_claims() |> sign()

  defp b64(data), do: Base.url_encode64(data, padding: false)
end
