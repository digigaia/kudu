import json
import shlex

from loguru import logger
import duct
import pytest

from kudu.wallet import wallet
import kudu

BOOTSTRAP = True
VERBOSE = False

# FIXME: use different container name than default for tests, also use different ports
# CONTAINER_NAME = 'kudupy_test'
KUDUNE_CMD = f'kudune {"-vv " if VERBOSE else ""}'  # --container {CONTAINER_NAME}'


def kudune(cmd, capture=False):
    cmd = duct.cmd(*shlex.split(KUDUNE_CMD), *shlex.split(cmd))
    print(f'executing {cmd}')
    if capture:
        return cmd.read()
    else:
        return cmd.run()


@pytest.fixture(scope='module')
def bootstrap_node():
    # create a fresh node from scratch
    kudune('destroy')
    kudune('start-node')
    kudune('bootstrap')


@pytest.fixture(scope='module')
def existing_node():
    # start an existing node
    kudune('start-node')


start_node = bootstrap_node if BOOTSTRAP else existing_node


@pytest.fixture(scope='module')
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



def eos_log(response, console_output):
    # this is a separate function (instead of inline) so it shows that the logs come from nodeos
    if console_output:
        logger.debug(f'Console output for tx {response["transaction_id"]}:')
    for output in console_output:
        for line in output.splitlines():
            logger.debug(line)


def push_action(client, actor, contract, action, args, exception_type=None):
    logger.debug(f'==== ACTION: {actor} {contract} {action} {args}')

    # create a new transaction with the given action
    action = kudu.Action(contract, action, kudu.PermissionLevel(actor, 'active'), args)
    tx = kudu.chain.Transaction({'actions': [action.to_dict()]})
    tx.link(client)
    # print(f'Linked tx: {pformat(tx.to_dict())}')

    # sign the transaction using the corresponding private key
    pkey = wallet.private_keys[actor]
    key = kudu.crypto.PrivateKey(pkey)

    signed_tx = tx.sign(key)
    # pprint(signed_tx.to_dict())

    # send the transaction
    result = signed_tx.send()
    logger.trace(json.dumps(result, indent=4))

    # parse result
    if 'processed' in result:
        # print console output
        console_output = []
        for action in result['processed']['action_traces']:
            if output := action.get('console'):
                console_output.append(output)
            for trace in action.get('inline_traces', []):
                if output := trace.get('console'):
                    console_output.append(output)
        eos_log(result, console_output)

    elif 'error' in result:
        err = result['error']
        msg = err['details'][0]['message']
        logger.error(err['what'])
        logger.error(msg)

        if exception_type is not None:
            raise exception_type(msg)

    else:
        msg = 'unhandled case!!!'
        logger.error(msg)
        if exception_type is not None:
            raise exception_type(msg)


def run_action_list(client, actions):
    for permission, contract, action, args in actions:
        logger.info(f'ACTION: {permission:12} {contract:>12}::{action:12} -  {args}')
        push_action(client, permission, contract, action, args)


def populate_db(client):
    # actions = list of tuples: (permission, contract, action, args)

    tokenacct = 'eosio.token'


    #### 1- create accounts ################################################
    if BOOTSTRAP:
        kudune('system-newaccount figaro eosio')
        kudune('system-newaccount sabin eosio')
        kudune('system-newaccount edgar eosio')
    wallet.import_from_kudune_wallet(['eosio', 'eosio.token', 'figaro', 'sabin', 'edgar'])

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

    tokenacct = 'eosio.token'

    def get_balance(account, symbol) -> float | None:
        b = client.v1.chain.get_currency_balance(account=account, code=tokenacct, symbol=symbol)
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
