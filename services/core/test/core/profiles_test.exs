defmodule Core.ProfilesTest do
  use Core.DataCase, async: true

  alias Core.Profiles
  alias Core.Profiles.UserProfile

  test "crea y luego actualiza el nombre visible con version 0 (D-29)" do
    user = Ecto.UUID.generate()

    assert {:ok, %UserProfile{display_name: "Ana", version: 0}} =
             Profiles.put_display_name(user, %{"display_name" => " Ana "})

    assert {:ok, %UserProfile{display_name: "Ana López"}} =
             Profiles.put_display_name(user, %{"display_name" => "Ana López"})

    assert Repo.get!(UserProfile, user).display_name == "Ana López"
  end

  test "valida longitud" do
    user = Ecto.UUID.generate()

    assert {:error, changeset} = Profiles.put_display_name(user, %{"display_name" => "  "})
    assert %{display_name: [_]} = errors_on(changeset)

    long = String.duplicate("a", 101)
    assert {:error, changeset} = Profiles.put_display_name(user, %{"display_name" => long})
    assert %{display_name: [_]} = errors_on(changeset)
  end

  test "no sobrescribe un perfil publicado por Auth (version > 0)" do
    user = Ecto.UUID.generate()

    Repo.insert!(%UserProfile{
      user_id: user,
      display_name: "Desde Auth",
      version: 3,
      updated_at: DateTime.utc_now()
    })

    assert {:error, :managed_by_auth} =
             Profiles.put_display_name(user, %{"display_name" => "Otro"})

    assert Repo.get!(UserProfile, user).display_name == "Desde Auth"
  end
end
