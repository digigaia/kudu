import json

import pytest

from kudu.chain import Action, PermissionLevel
import kudu


def test_submodule():
    assert str(kudu.APIClient) == "<class 'kudu.api.APIClient'>"
    assert str(kudu.chain.Action) == "<class 'kudu.chain.Action'>"


def test_name():
    name = kudu.Name('eosio')
    assert name == 'eosio'
    assert str(name) == 'eosio'
    assert repr(name) == "'eosio'"
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
    action = Action('eosio.token', 'transfer', PermissionLevel('eosio', 'active'), bytes.fromhex(
        '608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f'))
    assert action.account == 'eosio.token'
    assert action.name == 'transfer'
    assert action.authorization == [('eosio', 'active')]
    assert action.data.hex() == '608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f'
    with pytest.raises(AttributeError):
        action.authorization = 'forbidden'
    # assert str(action) == ""

    abi = kudu.abi.ABI(json.dumps(kudu.local.v1.chain.get_abi(account_name='eosio.token')['abi']))
    data = action.decode_data(abi)
    assert data == {'from': 'useraaaaaaaa', 'to': 'useraaaaaaab', 'quantity': '0.0001 SYS', 'memo': 'test memo'}
