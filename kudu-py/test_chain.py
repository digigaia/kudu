import shlex

from loguru import logger
import duct
import pytest

from kudu.chain import push_action
from kudu.wallet import wallet
import kudu

BOOTSTRAP = True
VERBOSE = False

# NOTE: use different container name than default for tests, also use different ports
CONTAINER_NAME = 'vaulta_pytest'
NODEOS_PORT = 8898

kudune = kudu.Kudune(container=CONTAINER_NAME, nodeos_port=NODEOS_PORT, verbose=VERBOSE)
wallet.set_kudune(kudune)


@pytest.fixture(scope='module')
def bootstrap_node():
    # create a fresh node from scratch
    kudune.destroy()
    kudune.start_node()
    kudune.bootstrap()


@pytest.fixture(scope='module')
def existing_node():
    # start an existing node
    kudune.start_node()


start_node = bootstrap_node if BOOTSTRAP else existing_node


@pytest.fixture(scope='module')
def node(start_node):
    # time.sleep(1)  # give it time to start
    yield
    kudune.stop_node()


@pytest.fixture
def client(node):
    return kudu.api.APIClient(f'http://localhost:{NODEOS_PORT}')


@pytest.fixture
def chain(client):
    return client.v1.chain


sysacct = 'core.vaulta'
tokenacct = 'eosio.token'


def test_is_running(chain):
    info = chain.get_info()
    assert info['head_block_num'] > 0


def run_action_list(client, actions):
    for actor, contract, action, args in actions:
        pkey = wallet.private_keys[actor]
        key = kudu.crypto.PrivateKey(pkey)

        logger.info(f'ACTION: {actor:12} {contract:>12}::{action:12} -  {args}')
        push_action(client, actor, key, contract, action, args)


def populate_db(client):
    # actions = list of tuples: (actor, contract, action, args)

    #### 1- create accounts ################################################
    if BOOTSTRAP:
        kudune.system_newaccount('figaro', 'eosio')
        kudune.system_newaccount('sabin', 'eosio')
        kudune.system_newaccount('edgar', 'eosio')
    wallet.import_from_kudune_wallet(['core.vaulta', 'eosio', 'eosio.token', 'figaro', 'sabin', 'edgar'])

    #### 2- token creation #################################################
    token_creation = [

        (tokenacct, tokenacct, 'create', dict(issuer='figaro',
                                              maximum_supply='1000000.000 GIL')),

        ('figaro',  tokenacct, 'issue',  dict(to='figaro',
                                              quantity='5000.000 GIL',
                                              memo='init token')),
    ]


    #### 3- initial token distribution #####################################
    distribution_actions = [

        ('figaro', tokenacct, 'transfer', {'from': 'figaro',
                                           'to': 'sabin',
                                           'quantity': '50.000 GIL',
                                           'memo': 'here you go!'}),

        ('figaro', tokenacct, 'transfer', {'from': 'figaro',
                                           'to': 'edgar',
                                           'quantity': '100.000 GIL',
                                           'memo': 'some for you too!'}),
    ]


    if BOOTSTRAP:
        run_action_list(client, token_creation)
        run_action_list(client, distribution_actions)


def test_new_token(client):
    populate_db(client)

    def get_balance(account, symbol) -> float | None:
        code = sysacct if symbol == 'A' else tokenacct
        b = client.v1.chain.get_currency_balance(account=account, code=code, symbol=symbol)
        # note: can also use the following to get all balances
        # b = client.v1.chain.get_table_rows(code=code, scope=account, table='accounts', json=True)
        if b:
            return float(b[0].split(' ')[0])
        else:
            return None

    edgar = get_balance('edgar', 'GIL')
    sabin = get_balance('sabin', 'GIL')
    print(f'Edgar: {edgar}\nSabin: {sabin}')

    transfer_actions = [
        ('edgar', tokenacct, 'transfer', {'from': 'edgar',
                                          'to': 'sabin',
                                          'quantity': '1.000 GIL',
                                          'memo': 'thank you!'}),
    ]
    run_action_list(client, transfer_actions)

    edgar_now = get_balance('edgar', 'GIL')
    sabin_now = get_balance('sabin', 'GIL')
    print(f'Edgar: {edgar_now}\nSabin: {sabin_now}')

    assert edgar_now == edgar - 1
    assert sabin_now == sabin + 1


def test_vaulta_transition(client):
    assert client.v1.chain.get_currency_balance(account='core.vaulta', code=sysacct, symbol='A') == ['2100000000.0000 A']
    wallet.import_from_kudune_wallet(['core.vaulta'])
    actions = [
        ('core.vaulta', sysacct, 'transfer', {'from': 'core.vaulta',
                                              'to': 'eosio',
                                              'quantity': '1.0000 A',
                                              'memo': 'hello'}),
    ]
    run_action_list(client, actions)
