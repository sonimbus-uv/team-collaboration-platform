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

  test "display_names/1 resuelve varios usuarios; nil sin perfil o eliminado" do
    [named, unnamed, deleted] = for _ <- 1..3, do: Ecto.UUID.generate()
    {:ok, _} = Profiles.put_display_name(named, %{"display_name" => "Ana"})
    {:ok, _} = Profiles.put_display_name(deleted, %{"display_name" => "Borrado"})

    Repo.update_all(from(p in UserProfile, where: p.user_id == ^deleted),
      set: [account_status: :deleted]
    )

    assert Profiles.display_names([named, unnamed, deleted, named]) ==
             %{named => "Ana", unnamed => nil, deleted => nil}

    assert Profiles.display_names([]) == %{}
  end
end
