import pytest

import kudu


@pytest.fixture
def client():
    # return kudu.api.APIClient('http://localhost:8888')
    return kudu.jungle


@pytest.fixture
def chain(client):
    return client.v1.chain


def test_api_client():
    assert isinstance(kudu.local, kudu.APIClient)
    assert isinstance(kudu.jungle, kudu.APIClient)


def test_get_info(chain):
    info = chain.get_info()
    assert info['head_block_num'] > 0


def test_get_abi(chain):
    abi = chain.get_abi(account_name='eosio')
    assert 'account_name' in abi
    assert 'abi' in abi
    assert abi['abi']['version'] == 'eosio::abi/1.2'


def test_push_transaction(chain):
    alice_son = chain.get_currency_balance(account='alice', code='eosio.token', symbol='SON')
    bob_son   = chain.get_currency_balance(account='bob',   code='eosio.token', symbol='SON')

    print(f'Alice has {alice_son} SON and Bob has {bob_son} SON')
    # assert both == 50

    args = {
        'from': 'alice',
        'to': 'bob',
        'quantity': '1.000 SON',
        'memo': 'yep!'
    }
    #args = {"from": "alice", "to": "bob", "quantity": "1.000 SON", "memo": "yep!"}
    args_encoded = bytes.fromhex('0000000000855c340000000000000e3de80300000000000003534f4e000000000479657021')

    action = kudu.Action('eosio.token', 'transfer', kudu.PermissionLevel('eosio', 'active'), args_encoded)
    action_dict = action.to_dict()
    print(f'Action: {action_dict}')

    tx = kudu.chain.Transaction({'actions': [action_dict]})
    print(f'Transaction: {tx.to_dict()}')

    key = kudu.crypto.PrivateKey('5JEc9CzLAx48Utvn7mo4y6hhmSVj7n4zgDNJx2KNZo3gSBr8Fet')
    # client = kudu.jungle
    tx.link(kudu.jungle)
    signed_tx = tx.sign_new(key)
    signed_tx.send()

    alice_son = chain.get_currency_balance(account='alice', code='eosio.token', symbol='SON')
    bob_son   = chain.get_currency_balance(account='bob',   code='eosio.token', symbol='SON')
    print(f'Alice has {alice_son} SON and Bob has {bob_son} SON')
    # alice transfer 1 SON to bob
    # assert alice has 49, bob 51

    pass
