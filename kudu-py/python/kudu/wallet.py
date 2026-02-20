#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pathlib import Path
from loguru import logger
# import pyntelope
import duct
# import invoke
import json
import shlex

import kudu
net = kudu.local.v1.chain

# FIXME!! refactor the whole file

def kudune(cmd, capture=False):
    cmd = duct.cmd('kudune', *shlex.split(cmd))
    if capture:
        return cmd.read()
    else:
        return cmd.run()


class Wallet(object):
    """This is an absolutely *insecure* wallet!!!

    DO NOT USE WITH SENSITIVE PRIVATE KEYS!!!
    """

    DEFAULT_WALLET_LOCATION = Path('~/.local/share/eos_dev_wallet/wallet.json').expanduser()

    def __init__(self, location=None):
        self.location = Path(location) if location else self.DEFAULT_WALLET_LOCATION
        self.public_keys = {}  # account -> pubkey
        self.private_keys = {}  # account -> privkey

    def _update_public_keys(self, accounts):
        for name in accounts:
            account = net.get_account(account_name=name)
            if 'error' in account:
                logger.warning(f'Account {name} doesn\'t exist')
                continue
            perms = account['permissions']
            for p in perms:
                if p['perm_name'] == 'active':
                    pubkey = p['required_auth']['keys'][0]['key']
                    action = 'Updating' if name in self.public_keys else 'Importing'
                    logger.info(f'{action} public key for account {name}: {pubkey}')
                    self.public_keys[name] = pubkey
                    self.private_keys.pop(name, None)
                    break
            else:
                logger.warning(f'Could not find public key associated with active permission for account {name}')

    def _import_private_keys(self, keypairs, accounts):
        for account, pubkey in self.public_keys.items():
            for pubkey2, privkey in keypairs:
                if pubkey == pubkey2:
                    logger.debug(f'Importing private key corresponding to {pubkey}')
                    self.private_keys[account] = privkey
                    break
            else:
                # we didn't find a corresponding private key for the public key
                # of the current account: only fail if it is an account we said
                # we wanted to import, otherwise go on normally
                if account in accounts:
                    logger.warning(f'Could not find private key for account: {account}')

    def import_from_kudune_wallet(self, accounts):
        # make sure we know the current public keys on the blockchain for the
        # accounts we are importing
        self._update_public_keys(accounts)

        # wallet_pwd = invoke.run('kudune wallet-password', hide=True).stdout
        wallet_pwd = kudune('--quiet wallet-password', capture=True)
        keypairs = json.loads(kudune(f'--quiet exec -- cleos wallet private_keys --password {wallet_pwd}',
                                     capture=True))

        self._import_private_keys(keypairs, accounts)
        self.save()

    def load(self):
        try:
            with open(self.location) as f:
                w = json.load(f)
                self.public_keys = w['public']
                self.private_keys = w['private']

        except FileNotFoundError:
            logger.warning(f'Wallet file does not exist: {self.location}')

    def save(self):
        w = {'public': self.public_keys,
             'private': self.private_keys}

        self.location.parent.mkdir(parents=True, exist_ok=True)
        with open(self.location, 'w') as f:
            json.dump(w, f, indent=4)

    def print(self):
        print('WALLET:')
        print('KNOWN ACCOUNTS:')
        for account, key in self.public_keys.items():
            print(f'  {account:12} => {key}')
        print('KNOWN ACCOUNTS WITH PRIVATE KEY:')
        for account, key in self.private_keys.items():
            print(f'  {account:12} => {key}')


wallet = Wallet()
wallet.load()
