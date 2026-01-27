import pytest

from kudu.chain import Action, PermissionLevel
import kudu


def test_submodule():
    assert str(kudu.APIClient) == "<class 'kudu.api.APIClient'>"
    assert str(kudu.chain.Action) == "<class 'kudu.chain.Action'>"


def test_name():
    name = kudu.Name('eosio')
    assert name == 'eosio'
    assert bytes(name).hex() == '0000000000ea3055'

    with pytest.raises(ValueError, match='normalized'):
        kudu.Name('2345;[h')

    with pytest.raises(ValueError, match='longer than 13 characters'):
        kudu.Name('123456789012345')


def test_permission_level():
    perm = kudu.chain.PermissionLevel('eosio', 'active')
    assert str(perm) == '<kudu.chain.PermissionLevel: eosio@active>'
    assert perm.actor == 'eosio'
    assert perm.permission == 'active'
    assert perm == kudu.chain.PermissionLevel('eosio', 'active')
    assert perm == ('eosio', 'active')
    assert perm == {'actor': 'eosio', 'permission': 'active'}
    assert perm != {'actor': 23, 'permission': None}
    assert bytes(perm).hex() == '0000000000ea305500000000a8ed3232'


def test_action():
    action = Action('eosio.token', 'transfer', PermissionLevel('eosio', 'active'), b"")
    assert action.account == 'eosio.token'
    assert action.name == 'transfer'
    assert action.authorization == [('eosio', 'active')]
    assert action.data == b""  # FIXME!!
    with pytest.raises(AttributeError):
        action.authorization = 'forbidden'
    # assert str(action) == ""
