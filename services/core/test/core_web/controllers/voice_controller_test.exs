defmodule CoreWeb.VoiceControllerTest do
  use CoreWeb.ConnCase, async: false

  import Core.Fixtures

  @api_key "test_livekit_key"
  @api_secret "test_livekit_secret_with_at_least_32_chars"
  @url "ws://livekit.test:7880"

  setup %{conn: conn} do
    previous = Application.get_env(:core, Core.Voice)
    Application.put_env(:core, Core.Voice, api_key: @api_key, api_secret: @api_secret, url: @url)

    on_exit(fn ->
      if previous,
        do: Application.put_env(:core, Core.Voice, previous),
        else: Application.delete_env(:core, Core.Voice)
    end)

    owner = user_id()
    group = group_fixture(owner)
    voice = channel_fixture(group, owner, %{"name" => "voz", "type" => "voice"})
    %{owner: owner, group: group, voice: voice, conn: conn}
  end

  defp request(conn, user, channel_id) do
    conn |> authenticate(user) |> post(~p"/api/v1/channels/#{channel_id}/voice-token")
  end

  defp decode(token) do
    [header, payload, signature] = String.split(token, ".")

    {Jason.decode!(Base.url_decode64!(header, padding: false)),
     Jason.decode!(Base.url_decode64!(payload, padding: false)), signature,
     header <> "." <> payload}
  end

  test "without authentication responds 401", %{conn: conn, voice: voice} do
    conn = post(conn, ~p"/api/v1/channels/#{voice.id}/voice-token")
    assert %{"status" => 401} = json_response(conn, 401)
  end

  test "member receives a token with the contract shape", %{
    conn: conn,
    owner: owner,
    voice: voice
  } do
    before = System.system_time(:second)
    conn = request(conn, owner, voice.id)

    assert %{"token" => token, "url" => @url, "expires_at" => expires_at} =
             json_response(conn, 200)["data"]

    assert ["no-store"] = get_resp_header(conn, "cache-control")

    {:ok, expires, 0} = DateTime.from_iso8601(expires_at)
    assert_in_delta DateTime.to_unix(expires), before + 300, 3

    {header, claims, signature, signing_input} = decode(token)
    assert header == %{"alg" => "HS256", "typ" => "JWT"}

    expected =
      :crypto.mac(:hmac, :sha256, @api_secret, signing_input)
      |> Base.url_encode64(padding: false)

    assert signature == expected

    assert claims["iss"] == @api_key
    assert claims["sub"] == owner
    assert claims["identity"] == owner
    assert claims["exp"] - claims["nbf"] == 300
    assert_in_delta claims["exp"], before + 300, 3

    assert claims["video"] == %{
             "room" => voice.id,
             "roomJoin" => true,
             "canSubscribe" => true,
             "canPublish" => true,
             "canPublishSources" => ["microphone", "camera"],
             "canPublishData" => false
           }
  end

  test "includes the display name when the profile has one", %{
    conn: conn,
    owner: owner,
    voice: voice
  } do
    {:ok, _} = Core.Profiles.put_display_name(owner, %{"display_name" => "Ana"})
    token = json_response(request(conn, owner, voice.id), 200)["data"]["token"]
    assert {_, %{"name" => "Ana"}, _, _} = decode(token)
  end

  test "omits the name when there is no profile", %{conn: conn, owner: owner, voice: voice} do
    token = json_response(request(conn, owner, voice.id), 200)["data"]["token"]
    {_, claims, _, _} = decode(token)
    refute Map.has_key?(claims, "name")
  end

  test "each call issues a new token and identity never comes from the client", %{
    conn: conn,
    group: group,
    owner: owner,
    voice: voice
  } do
    guest = user_id()
    member_fixture(group, guest, :member)

    conn =
      conn
      |> authenticate(guest)
      |> post(~p"/api/v1/channels/#{voice.id}/voice-token", %{
        "identity" => owner,
        "sub" => owner,
        "room" => "other"
      })

    {_, claims, _, _} = decode(json_response(conn, 200)["data"]["token"])
    assert claims["sub"] == guest
    assert claims["identity"] == guest
    assert claims["video"]["room"] == voice.id
  end

  test "non member gets 404", %{conn: conn, voice: voice} do
    conn = request(conn, user_id(), voice.id)
    assert %{"type" => "/problems/not-found", "status" => 404} = json_response(conn, 404)
  end

  test "unknown or malformed channel gets 404", %{conn: conn, owner: owner} do
    assert json_response(request(conn, owner, Ecto.UUID.generate()), 404)
    assert json_response(request(conn, owner, "not-a-uuid"), 404)
  end

  test "role below min_role gets 404", %{conn: conn, group: group, owner: owner} do
    restricted =
      channel_fixture(group, owner, %{"name" => "staff", "type" => "voice", "min_role" => "admin"})

    member = user_id()
    member_fixture(group, member, :member)

    assert %{"status" => 404} = json_response(request(conn, member, restricted.id), 404)
    assert json_response(request(conn, owner, restricted.id), 200)
  end

  test "text channel gets 422", %{conn: conn, group: group, owner: owner} do
    text = channel_fixture(group, owner, %{"name" => "texto", "type" => "text"})
    conn = request(conn, owner, text.id)

    assert %{"type" => "/problems/not-a-voice-channel", "status" => 422} =
             json_response(conn, 422)
  end

  test "archived group gets 409", %{conn: conn, group: group, owner: owner, voice: voice} do
    group
    |> Ecto.Changeset.change(archived_at: DateTime.utc_now())
    |> Core.Repo.update!()

    conn = request(conn, owner, voice.id)
    assert %{"type" => "/problems/group-archived", "status" => 409} = json_response(conn, 409)
  end

  for {label, config} <- [
        missing: nil,
        missing_key: [api_secret: @api_secret, url: @url],
        missing_url: [api_key: @api_key, api_secret: @api_secret],
        short_secret: [api_key: @api_key, api_secret: "short", url: @url]
      ] do
    test "#{label} configuration gets 503 without leaking details", %{
      conn: conn,
      owner: owner,
      voice: voice
    } do
      case unquote(Macro.escape(config)) do
        nil -> Application.delete_env(:core, Core.Voice)
        config -> Application.put_env(:core, Core.Voice, config)
      end

      conn = request(conn, owner, voice.id)

      assert %{"type" => "/problems/voice-unavailable", "status" => 503} =
               body = json_response(conn, 503)

      refute inspect(body) =~ @api_secret
    end
  end

  test "authorization is checked before configuration", %{conn: conn, voice: voice} do
    Application.delete_env(:core, Core.Voice)
    assert json_response(request(conn, user_id(), voice.id), 404)
  end
end
