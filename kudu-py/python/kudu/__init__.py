from .kudu import *  # noqa: F403
from kudu.api import APIClient
from kudu.chain import PermissionLevel, Action  # noqa: F401

class SubcommandProxy():
    def __init__(self, c: APIClient, path: list[str]):
        self.client = c
        self.path = path

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
