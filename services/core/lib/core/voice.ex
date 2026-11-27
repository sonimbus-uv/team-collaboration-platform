defmodule Core.Voice do
  alias Core.{Community, Profiles}
  alias Core.Community.{Channel, Group}

  @ttl_seconds 300
  @min_secret_bytes 32
  @sources ["microphone", "camera"]

  @type issued :: %{token: String.t(), url: String.t(), expires_at: DateTime.t()}

  @spec issue_token(term(), Ecto.UUID.t(), keyword()) ::
          {:ok, issued()}
          | {:error, :not_found | :group_archived | :not_voice_channel | :voice_unavailable}
  def issue_token(channel_id, user_id, opts \\ []) do
    now = Keyword.get_lazy(opts, :now, fn -> System.system_time(:second) end)

    with {:ok, %Channel{} = channel, %Group{} = group, _role} <-
           Community.get_channel_for_member(channel_id, user_id),
         :ok <- require_active(group),
         :ok <- require_voice(channel),
         {:ok, config} <- fetch_config() do
      claims = claims(config, channel, user_id, now)

      {:ok,
       %{
         token: sign(claims, config.api_secret),
         url: config.url,
         expires_at: DateTime.from_unix!(now + @ttl_seconds)
       }}
    end
  end

  def ttl_seconds, do: @ttl_seconds

  defp require_active(%Group{archived_at: nil}), do: :ok
  defp require_active(%Group{}), do: {:error, :group_archived}

  defp require_voice(%Channel{type: :voice}), do: :ok
  defp require_voice(%Channel{}), do: {:error, :not_voice_channel}

  defp fetch_config do
    config = Application.get_env(:core, __MODULE__, [])

    with api_key when is_binary(api_key) and api_key != "" <- config[:api_key],
         api_secret when is_binary(api_secret) and byte_size(api_secret) >= @min_secret_bytes <-
           config[:api_secret],
         url when is_binary(url) and url != "" <- config[:url] do
      {:ok, %{api_key: api_key, api_secret: api_secret, url: url}}
    else
      _ -> {:error, :voice_unavailable}
    end
  end

  defp claims(config, channel, user_id, now) do
    base = %{
      "iss" => config.api_key,
      "sub" => user_id,
      "identity" => user_id,
      "nbf" => now,
      "exp" => now + @ttl_seconds,
      "video" => %{
        "room" => channel.id,
        "roomJoin" => true,
        "canSubscribe" => true,
        "canPublish" => true,
        "canPublishSources" => @sources,
        "canPublishData" => false
      }
    }

    case Profiles.display_names([user_id]) do
      %{^user_id => name} when is_binary(name) and name != "" -> Map.put(base, "name", name)
      _ -> base
    end
  end

  defp sign(claims, secret) do
    header = %{"alg" => "HS256", "typ" => "JWT"}
    signing_input = encode(header) <> "." <> encode(claims)
    signature = :crypto.mac(:hmac, :sha256, secret, signing_input)
    signing_input <> "." <> Base.url_encode64(signature, padding: false)
  end

  defp encode(map), do: map |> Jason.encode!() |> Base.url_encode64(padding: false)
end
