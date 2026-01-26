import pytest

import kudu

# @pytest.fixture
# def client():
#     return kudu.APIClient('http://localhost:8888')

# @pytest.fixture
# def chain(client):
#     return client.v1.chain

# @pytest.mark.skip('TODO: fix this test!')
def test_submodule():
    assert str(kudu.APIClient) == "<class 'kudu.APIClient'>"
    assert str(kudu.chain.Action) == "<class 'kudu.chain.Action'>"


def test_api_client():
    assert isinstance(kudu.local, kudu.APIClient)
    assert isinstance(kudu.jungle, kudu.APIClient)

def test_base_types():
    perm = kudu.chain.PermissionLevel('eosio', 'active')
    assert str(perm) == '<kudu.chain.PermissionLevel: eosio@active>'
    assert perm.actor == 'eosio'
    assert perm.permission == 'active'
