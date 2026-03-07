#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from pathlib import Path
from loguru import logger
import duct
import json
import shlex

import kudu

# FIXME!! refactor the whole file
#         also make sure we don't have a wallet singleton lying around, do it properly

# def kudune(cmd, capture=False):
#     cmd = duct.cmd('kudune', *shlex.split(cmd))
#     if capture:
#         return cmd.read()
#     else:
#         return cmd.run()


class Wallet(object):
    """This is an absolutely *insecure* wallet!!!

    DO NOT USE WITH SENSITIVE PRIVATE KEYS!!!
    """

    DEFAULT_WALLET_LOCATION = Path('~/.local/share/eos_dev_wallet/wallet.json').expanduser()

    def __init__(self, location=None, chain_api=None):
        self.location = Path(location) if location else self.DEFAULT_WALLET_LOCATION
        self.public_keys = {}  # account -> pubkey
        self.private_keys = {}  # account -> privkey
        self.chain_api = chain_api or kudu.local.v1.chain

    def set_kudune(self, kudune: kudu.Kudune):
        self.kudune = kudune
        self.chain_api = kudu.APIClient(f'http://localhost:{kudune.nodeos_port}').v1.chain

    def _update_public_keys(self, accounts):
        for name in accounts:
            account = self.chain_api.get_account(account_name=name)
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

        wallet_pwd = self.kudune.wallet_password(quiet=True, capture=True)
        keypairs = json.loads(self.kudune.exec(f'cleos wallet private_keys --password {wallet_pwd}', quiet=True, capture=True))

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
