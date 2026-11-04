defmodule Core.CommunityTest do
  use Core.DataCase, async: true

  import Core.Fixtures

  alias Core.Community
  alias Core.Community.{Ban, Invitation, Member}

  describe "create_group/2" do
    test "el creador queda como admin en la misma transacción" do
      owner = user_id()

      assert {:ok, group} = Community.create_group(owner, %{"name" => "  Equipo  "})
      assert group.name == "Equipo"
      assert group.created_by == owner
      assert %Member{role: :admin} = Repo.get_by!(Member, group_id: group.id, user_id: owner)
    end

    test "rechaza un nombre fuera de 2–100 caracteres y no deja membresías" do
      owner = user_id()

      assert {:error, changeset} = Community.create_group(owner, %{"name" => "x"})
      assert %{name: [_]} = errors_on(changeset)
      refute Repo.exists?(from m in Member, where: m.user_id == ^owner)
    end

    test "lista solo los grupos del usuario, con su rol" do
      owner = user_id()
      group = group_fixture(owner)
      _other = group_fixture(user_id())

      assert [{listed, :admin}] = Community.list_groups_for_user(owner)
      assert listed.id == group.id
    end
  end

  describe "create_invitation/3" do
    setup do
      owner = user_id()
      %{owner: owner, group: group_fixture(owner)}
    end

    test "usa 7 días y usos ilimitados por defecto", %{owner: owner, group: group} do
      assert {:ok, invitation} = Community.create_invitation(group.id, owner, %{})
      assert invitation.max_uses == nil
      assert DateTime.diff(invitation.expires_at, invitation.created_at) == 7 * 86_400
      assert byte_size(invitation.code) == 16
    end

    test "valida caducidad y usos (D-22)", %{owner: owner, group: group} do
      assert {:error, changeset} =
               Community.create_invitation(group.id, owner, %{
                 "expires_in" => 60,
                 "max_uses" => 0
               })

      assert %{expires_in: [_], max_uses: [_]} = errors_on(changeset)
    end

    test "un moderador puede crear; un miembro no", %{owner: owner, group: group} do
      moderator = user_id()
      member = user_id()
      member_fixture(group, moderator, :moderator)
      member_fixture(group, member, :member)

      assert {:ok, _} = Community.create_invitation(group.id, moderator, %{})
      assert {:error, :forbidden} = Community.create_invitation(group.id, member, %{})
      assert {:ok, _} = Community.create_invitation(group.id, owner, %{"max_uses" => 5})
    end

    test "quien no es miembro recibe :not_found", %{group: group} do
      assert {:error, :not_found} = Community.create_invitation(group.id, user_id(), %{})
      assert {:error, :not_found} = Community.create_invitation("no-es-uuid", user_id(), %{})
    end
  end

  describe "join_group/2" do
    setup do
      owner = user_id()
      group = group_fixture(owner)
      %{owner: owner, group: group}
    end

    test "agrega como miembro y descuenta un uso", %{owner: owner, group: group} do
      invitation = invitation_fixture(group, owner, %{"max_uses" => 2})
      user = user_id()

      assert {:ok, :joined, joined, :member} = Community.join_group(invitation.code, user)
      assert joined.id == group.id
      assert Repo.get!(Invitation, invitation.id).uses == 1
      assert %Member{role: :member} = Repo.get_by!(Member, group_id: group.id, user_id: user)
    end

    test "ya miembro: no duplica ni descuenta (EX-02)", %{owner: owner, group: group} do
      invitation = invitation_fixture(group, owner, %{"max_uses" => 2})

      assert {:ok, :already_member, _group, :admin} = Community.join_group(invitation.code, owner)
      assert Repo.get!(Invitation, invitation.id).uses == 0
    end

    test "agotada: rechaza al siguiente usuario", %{owner: owner, group: group} do
      invitation = invitation_fixture(group, owner, %{"max_uses" => 1})
      second = user_id()

      assert {:ok, :joined, _, _} = Community.join_group(invitation.code, user_id())
      assert {:error, :invalid_invitation} = Community.join_group(invitation.code, second)
      assert Repo.get!(Invitation, invitation.id).uses == 1
      refute Repo.get_by(Member, group_id: group.id, user_id: second)
    end

    test "revocada o caducada: rechaza (EX-01)", %{owner: owner, group: group} do
      revoked = invitation_fixture(group, owner)
      expired = invitation_fixture(group, owner)
      past = DateTime.add(DateTime.utc_now(), -1, :second)

      Repo.update_all(from(i in Invitation, where: i.id == ^revoked.id),
        set: [revoked_at: DateTime.utc_now()]
      )

      # Se mueve también created_at para respetar el CHECK de caducidad.
      Repo.update_all(from(i in Invitation, where: i.id == ^expired.id),
        set: [expires_at: past, created_at: DateTime.add(past, -3_600, :second)]
      )

      assert {:error, :invalid_invitation} = Community.join_group(revoked.code, user_id())
      assert {:error, :invalid_invitation} = Community.join_group(expired.code, user_id())
      assert {:error, :invalid_invitation} = Community.join_group("no-existe", user_id())
    end

    test "usuario bloqueado: rechaza sin descontar (EX-03)", %{owner: owner, group: group} do
      invitation = invitation_fixture(group, owner)
      banned = user_id()
      Repo.insert!(%Ban{group_id: group.id, user_id: banned, banned_by: owner})

      assert {:error, :banned} = Community.join_group(invitation.code, banned)
      assert Repo.get!(Invitation, invitation.id).uses == 0
      refute Repo.get_by(Member, group_id: group.id, user_id: banned)
    end
  end

  describe "create_channel/3" do
    setup do
      owner = user_id()
      %{owner: owner, group: group_fixture(owner)}
    end

    test "crea un canal de texto o de voz", %{owner: owner, group: group} do
      assert {:ok, channel} =
               Community.create_channel(group.id, owner, %{"name" => "general", "type" => "text"})

      assert channel.min_role == :member

      assert {:ok, %{type: :voice}} =
               Community.create_channel(group.id, owner, %{"name" => "sala", "type" => "voice"})
    end

    test "rechaza nombre repetido entre canales vigentes (EX-01)", %{owner: owner, group: group} do
      channel_fixture(group, owner, %{"name" => "general"})

      assert {:error, changeset} =
               Community.create_channel(group.id, owner, %{"name" => "general", "type" => "text"})

      assert %{name: ["has already been taken"]} = errors_on(changeset)
    end

    test "valida el formato del nombre", %{owner: owner, group: group} do
      assert {:error, changeset} =
               Community.create_channel(group.id, owner, %{
                 "name" => "Con Espacios",
                 "type" => "text"
               })

      assert %{name: [_]} = errors_on(changeset)
    end

    test "solo un admin crea canales", %{group: group} do
      moderator = user_id()
      member_fixture(group, moderator, :moderator)

      assert {:error, :forbidden} =
               Community.create_channel(group.id, moderator, %{"name" => "x", "type" => "text"})
    end

    test "list_channels/2 respeta min_role (D-17)", %{owner: owner, group: group} do
      member = user_id()
      member_fixture(group, member, :member)
      channel_fixture(group, owner, %{"name" => "general"})
      channel_fixture(group, owner, %{"name" => "staff", "min_role" => "moderator"})

      assert {:ok, [%{name: "general"}]} = Community.list_channels(group.id, member)
      assert {:ok, [_, _]} = Community.list_channels(group.id, owner)
    end
  end

  describe "list_members/2" do
    test "devuelve rol y nombre visible; nil sin perfil o con cuenta eliminada" do
      owner = user_id()
      group = group_fixture(owner)
      named = user_id()
      unnamed = user_id()
      deleted = user_id()

      for user <- [named, unnamed, deleted], do: member_fixture(group, user, :member)

      {:ok, _} = Core.Profiles.put_display_name(owner, %{"display_name" => "Admin"})
      {:ok, _} = Core.Profiles.put_display_name(named, %{"display_name" => "Ana"})
      {:ok, _} = Core.Profiles.put_display_name(deleted, %{"display_name" => "Borrado"})

      Repo.update_all(
        from(p in Core.Profiles.UserProfile, where: p.user_id == ^deleted),
        set: [account_status: :deleted]
      )

      assert {:ok, [first | rest]} = Community.list_members(group.id, named)
      assert %{user_id: ^owner, role: :admin, display_name: "Admin"} = first

      by_user = Map.new(rest, &{&1.user_id, &1})
      assert by_user[named].display_name == "Ana"
      assert by_user[unnamed].display_name == nil
      assert by_user[deleted].display_name == nil
      refute Map.has_key?(by_user[deleted], :account_status)
    end

    test "quien no es miembro recibe :not_found" do
      group = group_fixture(user_id())
      assert {:error, :not_found} = Community.list_members(group.id, user_id())
    end
  end
end
