import pytest

import kudu

@pytest.fixture
def client():
    return kudu.APIClient('http://localhost:8888')

@pytest.fixture
def chain(client):
    return client.v1.chain

def test_get_info(chain):
    info = chain.get_info()
    assert info['head_block_num'] > 0

def test_get_abi(chain):
    abi = chain.get_abi(account_name='eosio')
    assert 'account_name' in abi
    assert 'abi' in abi
    assert abi['abi']['version'] == 'eosio::abi/1.2'
