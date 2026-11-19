defmodule Core.AccessTokenTest do
  use ExUnit.Case, async: true

  import Core.AccessTokenHelpers

  alias Core.AccessToken

  @now 1_792_627_200

  setup do
    user = Ecto.UUID.generate()
    %{user: user, claims: valid_claims(user, @now)}
  end

  defp verify(token, now \\ @now), do: AccessToken.verify(token, now: now)

  test "acepta un token válido y devuelve sub, sid y exp", %{user: user, claims: claims} do
    assert {:ok, %{user_id: ^user, session_id: sid, expires_at: exp}} = verify(sign(claims))
    assert sid == claims["sid"]
    assert exp == @now + 900
  end

  describe "encabezado" do
    test "rechaza otro alg, otro typ, sin kid o con crit/jku", %{claims: claims} do
      for header <- [
            %{default_header() | "alg" => "HS256"},
            %{default_header() | "alg" => "none"},
            %{default_header() | "typ" => "JWT"},
            Map.delete(default_header(), "kid"),
            Map.put(default_header(), "crit", ["exp"]),
            Map.put(default_header(), "jku", "https://evil.invalid/jwks.json")
          ] do
        assert {:error, :invalid} = verify(sign(claims, header)), inspect(header)
      end
    end

    test "rechaza un kid que no está en el JWKS", %{claims: claims} do
      header = %{default_header() | "kid" => "otro-kid"}
      assert {:error, :invalid} = verify(sign(claims, header))
    end
  end

  describe "firma" do
    test "rechaza una firma de otra clave", %{claims: claims} do
      {_public, other_private} = :crypto.generate_key(:eddsa, :ed25519)

      token =
        sign_raw(Jason.encode!(default_header()), Jason.encode!(claims), other_private)

      assert {:error, :invalid} = verify(token)
    end

    test "rechaza claims alterados después de firmar", %{claims: claims} do
      [h, _p, s] = claims |> sign() |> String.split(".")

      forged =
        Base.url_encode64(Jason.encode!(%{claims | "sub" => Ecto.UUID.generate()}),
          padding: false
        )

      assert {:error, :invalid} = verify(Enum.join([h, forged, s], "."))
    end

    test "rechaza tokens mal formados" do
      for token <- ["", "a.b", "a.b.c.d", "###.###.###", String.duplicate("a", 9_000)] do
        assert {:error, :invalid} = verify(token)
      end
    end
  end

  describe "claims" do
    test "rechaza issuer o audiencia incorrectos", %{claims: claims} do
      assert {:error, :invalid} = verify(sign(%{claims | "iss" => "https://otro.invalid"}))
      assert {:error, :invalid} = verify(sign(%{claims | "aud" => ["sonimbus-files"]}))
      assert {:error, :invalid} = verify(sign(%{claims | "aud" => "sonimbus-core"}))
    end

    test "rechaza sub, sid o jti que no son UUID", %{claims: claims} do
      for key <- ["sub", "sid", "jti"] do
        assert {:error, :invalid} = verify(sign(%{claims | key => "no-uuid"})), key
      end

      assert {:error, :invalid} = verify(sign(%{claims | "sub" => String.upcase(claims["sub"])}))
    end

    test "rechaza claims faltantes", %{claims: claims} do
      for key <- Map.keys(claims) do
        assert {:error, :invalid} = verify(sign(Map.delete(claims, key))), key
      end
    end

    test "exp es estricto, sin gracia", %{claims: claims} do
      assert {:ok, _} = verify(sign(claims), @now + 899)
      assert {:error, :invalid} = verify(sign(claims), @now + 900)
    end

    test "exige iat < exp <= iat + 900 y enteros", %{claims: claims} do
      assert {:error, :invalid} = verify(sign(%{claims | "exp" => @now + 901}))
      assert {:error, :invalid} = verify(sign(%{claims | "exp" => @now}))
      assert {:error, :invalid} = verify(sign(%{claims | "exp" => @now + 900.0}))
    end

    test "tolera hasta 30 s de iat futuro", %{claims: claims} do
      future = %{claims | "iat" => @now + 30, "exp" => @now + 930}
      assert {:ok, _} = verify(sign(future))

      too_far = %{claims | "iat" => @now + 31, "exp" => @now + 931}
      assert {:error, :invalid} = verify(sign(too_far))
    end

    test "rechaza claves repetidas en los claims", %{claims: claims} do
      json = Jason.encode!(claims)
      duplicated = String.replace_suffix(json, "}", ~s(,"sub":"#{Ecto.UUID.generate()}"}))

      assert {:error, :invalid} = verify(sign_raw(Jason.encode!(default_header()), duplicated))
    end
  end
end
