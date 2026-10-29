defmodule Core.AccessToken do
  @moduledoc """
  Verificación del access JWT de Auth según `contracts/token-profile.md`
  (D-07, D-16).

  - Encabezado: `alg` exactamente `EdDSA`, `typ` `at+jwt` y `kid`. Se rechazan
    `crit`, `jku`, `x5u`, `jwk` y `x5c`: la clave sale solo del JWKS
    configurado (`Core.AccessToken.JWKS`).
  - Firma Ed25519 con `:crypto`, antes de mirar los claims.
  - Claims: `iss` exacto, `aud` como arreglo que incluye `sonimbus-core`,
    `sub`, `sid` y `jti` UUID, `iat` y `exp` enteros con
    `iat < exp <= iat + 900`, `exp` estricto y hasta 30 s de tolerancia solo
    para un `iat` futuro.
  - Campos duplicados en el encabezado o en los claims invalidan el token.

  La comprobación de la sesión (`sid`) en Auth no se hace todavía: es la
  desviación temporal D-30 (`docs/database/core-decisions-draft.md`).

  Nunca registra el token ni sus partes.
  """

  alias Core.AccessToken.JWKS

  @audience "sonimbus-core"
  @max_lifetime 900
  @iat_leeway 30
  @max_token_bytes 8_192
  @forbidden_header_params ~w(crit jku x5u jwk x5c)

  @type claims :: %{
          user_id: Ecto.UUID.t(),
          session_id: Ecto.UUID.t(),
          expires_at: integer()
        }

  @doc """
  Verifica `token`. Devuelve `{:ok, claims}`, `{:error, :invalid}` o
  `{:error, :unavailable}` cuando no se pudo obtener un JWKS vigente (el
  token no se demuestra inválido; la API responde 503).

  Opciones: `:now` (segundos Unix), `:issuer` y `:jwks` (servidor de caché).
  """
  @spec verify(binary(), keyword()) :: {:ok, claims()} | {:error, :invalid | :unavailable}
  def verify(token, opts \\ [])

  def verify(token, opts) when is_binary(token) and byte_size(token) <= @max_token_bytes do
    now = Keyword.get_lazy(opts, :now, fn -> System.system_time(:second) end)
    issuer = Keyword.get_lazy(opts, :issuer, &configured_issuer/0)
    jwks = Keyword.get(opts, :jwks, JWKS)

    with [h64, p64, s64] <- String.split(token, "."),
         {:ok, header} <- decode_json_segment(h64),
         {:ok, kid} <- validate_header(header),
         {:ok, signature} <- decode_segment(s64),
         {:ok, public_key} <- JWKS.get_key(jwks, kid),
         true <- valid_signature?(h64 <> "." <> p64, signature, public_key),
         {:ok, claims} <- decode_json_segment(p64),
         {:ok, claims} <- validate_claims(claims, issuer, now) do
      {:ok, claims}
    else
      {:error, :unavailable} -> {:error, :unavailable}
      _ -> {:error, :invalid}
    end
  end

  def verify(_token, _opts), do: {:error, :invalid}

  @doc "Audiencia que Core exige en `aud`."
  def audience, do: @audience

  defp configured_issuer do
    Application.fetch_env!(:core, __MODULE__) |> Keyword.fetch!(:issuer)
  end

  defp validate_header(%{"alg" => "EdDSA", "typ" => "at+jwt", "kid" => kid} = header)
       when is_binary(kid) and kid != "" do
    if Enum.any?(@forbidden_header_params, &Map.has_key?(header, &1)),
      do: :error,
      else: {:ok, kid}
  end

  defp validate_header(_header), do: :error

  defp valid_signature?(signing_input, signature, public_key) when byte_size(signature) == 64 do
    :crypto.verify(:eddsa, :none, signing_input, signature, [public_key, :ed25519])
  end

  defp valid_signature?(_signing_input, _signature, _public_key), do: false

  defp validate_claims(
         %{
           "iss" => iss,
           "aud" => aud,
           "sub" => sub,
           "sid" => sid,
           "iat" => iat,
           "exp" => exp,
           "jti" => jti
         },
         issuer,
         now
       )
       when is_binary(iss) and is_list(aud) and is_integer(iat) and is_integer(exp) do
    with true <- iss == issuer,
         true <- Enum.all?(aud, &is_binary/1) and @audience in aud,
         {:ok, user_id} <- uuid(sub),
         {:ok, session_id} <- uuid(sid),
         {:ok, _jti} <- uuid(jti),
         true <- iat < exp and exp <= iat + @max_lifetime,
         true <- iat <= now + @iat_leeway,
         true <- now < exp do
      {:ok, %{user_id: user_id, session_id: session_id, expires_at: exp}}
    else
      _ -> :error
    end
  end

  defp validate_claims(_claims, _issuer, _now), do: :error

  # Solo la forma canónica en minúsculas, como la emite Auth.
  defp uuid(value) when is_binary(value) do
    case Ecto.UUID.cast(value) do
      {:ok, ^value} -> {:ok, value}
      _ -> :error
    end
  end

  defp uuid(_value), do: :error

  defp decode_segment(segment) do
    case Base.url_decode64(segment, padding: false) do
      {:ok, bytes} -> {:ok, bytes}
      :error -> :error
    end
  end

  @doc false
  # JSON con objeto en la raíz y sin claves repetidas. Jason se queda con la
  # última clave repetida, así que se decodifica con objetos ordenados para
  # detectarlas.
  def decode_json_object(json) do
    case Jason.decode(json, objects: :ordered_objects) do
      {:ok, %Jason.OrderedObject{values: values}} ->
        keys = Enum.map(values, &elem(&1, 0))

        if length(keys) == length(Enum.uniq(keys)) do
          {:ok, Map.new(values, fn {key, value} -> {key, plain(value)} end)}
        else
          :error
        end

      _ ->
        :error
    end
  end

  defp decode_json_segment(segment) do
    with {:ok, json} <- decode_segment(segment), do: decode_json_object(json)
  end

  defp plain(%Jason.OrderedObject{values: values}),
    do: Map.new(values, fn {key, value} -> {key, plain(value)} end)

  defp plain(list) when is_list(list), do: Enum.map(list, &plain/1)
  defp plain(value), do: value
end
