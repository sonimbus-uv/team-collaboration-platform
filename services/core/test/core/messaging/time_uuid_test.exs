defmodule Core.Messaging.TimeUUIDTest do
  use ExUnit.Case, async: true

  alias Core.Messaging.TimeUUID

  test "genera UUID v1 con el instante dado" do
    at = ~U[2026-10-29 15:04:05.123456Z]
    uuid = TimeUUID.generate(at)

    assert <<_::binary-size(14), "1", _::binary>> = uuid
    assert {:ok, ^at} = TimeUUID.to_datetime(uuid)
  end

  test "dos UUID del mismo instante son distintos" do
    at = DateTime.utc_now()
    refute TimeUUID.generate(at) == TimeUUID.generate(at)
  end

  test "rechaza UUID que no son versión 1" do
    assert :error = TimeUUID.to_datetime(Ecto.UUID.generate())
    assert :error = TimeUUID.to_datetime("no-uuid")
  end

  test "buckets mensuales" do
    assert TimeUUID.bucket(~U[2026-10-29 00:00:00Z]) == 202_610
    assert TimeUUID.previous_bucket(202_610) == 202_609
    assert TimeUUID.previous_bucket(202_601) == 202_512
  end
end
