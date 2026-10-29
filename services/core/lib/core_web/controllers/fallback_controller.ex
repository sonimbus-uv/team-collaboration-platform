defmodule CoreWeb.FallbackController do
  @moduledoc """
  Traduce los errores de los contextos a Problem Details (`CoreWeb.Problem`).
  """
  use CoreWeb, :controller

  alias CoreWeb.Problem

  def call(conn, {:error, %Ecto.Changeset{} = changeset}) do
    if conflict?(changeset) do
      Problem.send_problem(conn, 409, "conflict", "Conflict",
        detail: "The resource conflicts with an existing one.",
        errors: Problem.changeset_errors(changeset)
      )
    else
      Problem.send_validation_problem(conn, changeset)
    end
  end

  def call(conn, {:error, :not_found}) do
    Problem.send_problem(conn, 404, "not-found", "Not found",
      detail: "The resource does not exist or you do not have access to it."
    )
  end

  def call(conn, {:error, :forbidden}) do
    Problem.send_problem(conn, 403, "forbidden", "Forbidden",
      detail: "Your role in this group does not allow this action."
    )
  end

  def call(conn, {:error, :group_archived}) do
    Problem.send_problem(conn, 409, "group-archived", "Group archived",
      detail: "The group is archived and is read-only."
    )
  end

  def call(conn, {:error, :invalid_invitation}) do
    Problem.send_problem(conn, 422, "invitation-invalid", "Invalid invitation",
      detail: "The invitation does not exist, has expired, was revoked or has no uses left."
    )
  end

  def call(conn, {:error, :banned}) do
    Problem.send_problem(conn, 403, "banned", "Banned from group",
      detail: "You cannot join this group."
    )
  end

  def call(conn, {:error, :managed_by_auth}) do
    Problem.send_problem(conn, 409, "profile-managed-by-auth", "Profile managed by Auth",
      detail: "The display name is now provided by the Auth service."
    )
  end

  # Un nombre de canal repetido (CU-13 EX-01) es un conflicto, no un dato
  # mal formado.
  defp conflict?(changeset) do
    Enum.any?(changeset.errors, fn {_field, {_msg, opts}} ->
      opts[:constraint] == :unique
    end)
  end
end
