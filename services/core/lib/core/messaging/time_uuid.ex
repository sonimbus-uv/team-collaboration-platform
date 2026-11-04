defmodule Core.Messaging.TimeUUID do
  @moduledoc """
  UUID versión 1 (timeuuid) generado por Core: define el orden de los
  mensajes (D-08). Los 14 bits de secuencia y los 48 de nodo son aleatorios
  (con el bit de multidifusión encendido, como pide RFC 9562 para nodos que
  no son una MAC), así que dos mensajes del mismo instante no chocan.
  """
  import Bitwise

  # Intervalos de 100 ns entre 1582-10-15 y 1970-01-01.
  @gregorian_offset 0x01B21DD213814000

  @doc "Nuevo timeuuid para el instante actual o para `datetime`."
  def generate(datetime \\ nil) do
    ticks = unix_ticks(datetime) + @gregorian_offset

    <<clock_seq::14, node::48, _::2>> = :crypto.strong_rand_bytes(8)
    node = node ||| 0x010000000000

    time_low = ticks &&& 0xFFFFFFFF
    time_mid = ticks >>> 32 &&& 0xFFFF
    time_hi = ticks >>> 48 &&& 0x0FFF

    {:ok, uuid} =
      Ecto.UUID.load(
        <<time_low::32, time_mid::16, 1::4, time_hi::12, 2::2, clock_seq::14, node::48>>
      )

    uuid
  end

  defp unix_ticks(nil), do: div(System.os_time(:nanosecond), 100)
  defp unix_ticks(%DateTime{} = datetime), do: DateTime.to_unix(datetime, :microsecond) * 10

  @doc "Instante UTC de un timeuuid, o `:error` si no es un UUID versión 1."
  def to_datetime(uuid) when is_binary(uuid) do
    case Ecto.UUID.dump(uuid) do
      {:ok, <<time_low::32, time_mid::16, 1::4, time_hi::12, 2::2, _::bitstring>>} ->
        ticks = time_hi <<< 48 ||| time_mid <<< 32 ||| time_low
        {:ok, DateTime.from_unix!(div(ticks - @gregorian_offset, 10), :microsecond)}

      _ ->
        :error
    end
  end

  def to_datetime(_uuid), do: :error

  @doc "Bucket mensual `AAAAMM` (UTC) de una fecha (`002_messages.cql`)."
  def bucket(%DateTime{year: year, month: month}), do: year * 100 + month

  @doc "Bucket del mes anterior."
  def previous_bucket(bucket) when rem(bucket, 100) == 1, do: (div(bucket, 100) - 1) * 100 + 12
  def previous_bucket(bucket), do: bucket - 1
end
