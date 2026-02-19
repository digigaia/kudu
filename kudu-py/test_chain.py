import json
import shlex

import duct
import pytest

import kudu

VERBOSE = False

CONTAINER_NAME = 'kudupy_test'
KUDUNE_CMD = f'kudune {"-vv " if VERBOSE else ""} --container {CONTAINER_NAME}'


def kudune(cmd, capture=False):
    # kudune_cmd = [*shlex.split(KUDUNE_CMD), *shlex.split(cmd)]
    cmd = duct.cmd(*shlex.split(KUDUNE_CMD), *shlex.split(cmd))
    print(f'executing {cmd}')
    if capture:
        return cmd.read()
    else:
        return cmd.run()


@pytest.fixture
def bootstrap():
    # create a fresh node from scratch
    kudune('destroy')
    kudune('start-node')
    kudune('bootstrap')


@pytest.fixture
def existing():
    # start an existing node
    kudune('start-node')

# start_node = bootstrap
start_node = existing

@pytest.fixture
def node(start_node):
    # time.sleep(1)  # give it time to start
    yield
    kudune('stop-node')


@pytest.fixture
def client(node):
    return kudu.api.APIClient('http://localhost:8888')


@pytest.fixture
def chain(client):
    return client.v1.chain


def test_is_running(chain):
    info = chain.get_info()
    assert info['head_block_num'] > 0


def test_new_token(chain):
    # kudune('system-newaccount sabin eosio')
    # kudune('system-newaccount edgar eosio')
    # logger.info('sonthegame ok!')
    # wallet.import_from_kudune_wallet(['eosio', 'eosio.token', 'sabin', 'edgar'])

    wallet_pwd = kudune('--quiet wallet-password', capture=True)
    print('PW {}', wallet_pwd)
    keypairs = json.loads(kudune(f'--quiet exec -- cleos wallet private_keys --password {wallet_pwd}',
                                 capture=True))
    print(keypairs)
