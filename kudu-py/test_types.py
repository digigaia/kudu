import pytest

from kudu.chain import Action, PermissionLevel, Transaction
from kudu.crypto import PrivateKey, PublicKey
import kudu


def test_submodule():
    assert str(kudu.APIClient) == "<class 'kudu.api.APIClient'>"
    assert str(kudu.chain.Action) == "<class 'kudu.chain.Action'>"


def test_name():
    name = kudu.Name('eosio')

    assert str(name) == 'eosio'
    assert repr(name) == '<kudu.Name: eosio>'
    assert bytes(name).hex() == '0000000000ea3055'

    assert name == 'eosio'

    with pytest.raises(ValueError, match='normalized'):
        kudu.Name('2345;[h')

    with pytest.raises(ValueError, match='longer than 13 characters'):
        kudu.Name('123456789012345')


def test_crypto():
    priv = PrivateKey('PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6')
    pub = PublicKey('EOS1111111111111111111111111111111114T1Anm')

    assert str(priv) == 'PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6'
    assert repr(priv) == '<kudu.PrivateKey: PVT_R1_PtoxLPzJZURZmPS4e26pjBiAn41mkkLPrET5qHnwDvbvqFEL6>'

    assert str(pub) == 'PUB_K1_11111111111111111111111111111111149Mr2R'
    assert repr(pub) == '<kudu.PublicKey: PUB_K1_11111111111111111111111111111111149Mr2R>'


def test_permission_level():
    perm = kudu.chain.PermissionLevel('eosio', 'active')

    assert str(perm) == 'eosio@active'
    assert repr(perm) == '<kudu.chain.PermissionLevel: eosio@active>'
    assert bytes(perm).hex() == '0000000000ea305500000000a8ed3232'

    assert perm.actor == 'eosio'
    assert perm.permission == 'active'
    assert perm == kudu.chain.PermissionLevel('eosio', 'active')
    assert perm == ('eosio', 'active')
    assert perm == {'actor': 'eosio', 'permission': 'active'}
    assert perm != {'actor': 23, 'permission': None}


# FIXME: "data" should be able to be passed as data json repr
ACTION = {
    "account": "eosio.token",
    "name": "transfer",
    "authorization": [
        {
            "actor": "useraaaaaaaa",
            "permission": "active"
        }
    ],
    "data": "608c31c6187315d6708c31c6187315d60100000000000000045359530000000000"
}

TX = {
    "expiration": "2018-06-27T20:33:54.000",
    "ref_block_num": 45323,
    "ref_block_prefix": 2628749070,
    "max_net_usage_words": 0,
    "max_cpu_usage_ms": 0,
    "delay_sec": 0,
    "context_free_actions": [],
    "actions": [ACTION],
    "transaction_extensions": [],
}

TX_HEX = 'b2f4335b0bb10e87af9c000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000'


def test_action():
    action = Action('eosio.token', 'transfer', PermissionLevel('eosio', 'active'), bytes.fromhex(
        '608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f'))
    # assert str(action) == '...'  # FIXME: implement me
    # assert repr, bytes
    assert action.account == 'eosio.token'
    assert action.name == 'transfer'
    assert action.authorization == [('eosio', 'active')]
    assert action.data.hex() == '608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f'

    encoded = {
        'account': 'eosio.token',
        'name': 'transfer',
        'authorization': [
            {
                'actor': 'eosio',
                'permission': 'active'
            }
        ],
        'data': '608c31c6187315d6708c31c6187315d6010000000000000004535953000000000974657374206d656d6f'
    }
    decoded_data = {'from': 'useraaaaaaaa', 'to': 'useraaaaaaab', 'quantity': '0.0001 SYS', 'memo': 'test memo'}
    decoded = encoded.copy()
    decoded['data'] = {
        'from': 'useraaaaaaaa',
        'to': 'useraaaaaaab',
        'quantity': '0.0001 SYS',
        'memo': 'test memo'
    }

    # we can decode encoded data
    data = action.decode_data()
    assert data == decoded_data

    # we can get a python object with the action data either encoded or decoded
    assert action.to_dict() == encoded
    assert action.decoded() == decoded

    # we can compare an Action with either a decoded or encoded dict
    assert action == encoded
    assert action == decoded

    with pytest.raises(AttributeError):
        action.authorization = 'forbidden'
    # assert str(action) == ""


def test_transaction():
    transaction = Transaction(TX)
    assert transaction.ref_block_num == 45323
    assert transaction.to_dict() == TX
    # assert transaction == TX   # FIXME: implement me!
    # assert str(transaction) == '...'

    assert len(transaction.actions) == 1
    assert isinstance(transaction.actions[0], Action)
    assert transaction.actions[0].name == 'transfer'
    assert transaction.actions[0] == ACTION

    assert bytes(transaction).hex() == TX_HEX

    with pytest.raises(ValueError):
        Transaction('this should fail gracefully')
