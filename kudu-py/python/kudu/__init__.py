import shlex

import duct

from .kudu import *  # noqa: F403
from kudu.api import APIClient
from kudu.chain import *  # noqa: F403
from kudu.crypto import *  # noqa: F403
from kudu.time import *  # noqa: F403

class SubcommandProxy():
    def __init__(self, c: APIClient, path: list[str]):
        self.client = c
        self.path = path

    def __repr__(self) -> str:
        return f'<kudu.SubcommandProxy: {self.client} - path: {self.path}>'


    def __getattr__(self, subpath: str) -> SubcommandProxy:  # noqa: F405
        return SubcommandProxy(self.client, [*self.path, subpath])

    def __call__(self, *args, **kwargs):
        path = '/' + '/'.join(self.path)
        if not args and not kwargs:
            # if we call the command without arguments, it's a GET
            return self.client.get(path)
        elif not kwargs and len(args) == 1 and isinstance(args[0], dict):
            # if we have a single argument that is a dict (the params), we need
            # to pass it as is to the underlying function
            return self.client.call(path, *args)
        elif not args:
            # if we have no positional argument, gather all the named ones in a
            # dict and use that as params for the underlying call
            return self.client.call(path, kwargs)
        else:
            # not sure what to do with mixed args and kwargs
            raise ValueError(f'Cannot call subcommand {path} with positional args, unless it is a dict of named args:'
                             f'pos: {args} - named: {kwargs}')


def apiclient_dynamic_get(c: APIClient, subpath: str):
    return SubcommandProxy(c, [subpath])


APIClient.__getattr__ = apiclient_dynamic_get



class KuduneCommand():
    def __init__(self, kudune, cmd: str):
        self.kudune = kudune
        self.cmd = cmd.replace('_', '-')

    def __call__(self, *args, capture=False, quiet=False):
        # allow to pass everything as a single string
        if len(args) == 1:
            args = shlex.split(args[0])

        cmd = ['kudune',
               '--ports', f'{self.kudune.nodeos_port}:8888',
               '--container', f'{self.kudune.container}']
        if self.kudune.verbose:
            cmd.append('-vv')
        if quiet:
            cmd.append('--quiet')
        cmd.append(self.cmd)
        if self.cmd == 'exec':
            # if we're executing a command, add a '--' to ensure command options are properly
            # passed through and not "stolen" by kudune
            cmd.append('--')
        cmd = duct.cmd(*cmd, *args)
        print(f'executing {cmd} with args: {args}')
        if capture:
            return cmd.read()
        else:
            return cmd.run()


class Kudune():
    def __init__(self,
                 container: str = 'vaulta_nodeos',
                 nodeos_port: int = 8888,
                 verbose: bool = False,
                 ):
        self.container = container
        self.nodeos_port = nodeos_port
        self.verbose = verbose

    def __getattr__(self, cmd):
        return KuduneCommand(self, cmd)
