It contains:

    general enhancements:
        we do not need to pass an ABIProvider as arg for those functions that require it, we have a static registry of ABIs which can queried automatically now, so APIs in general are much cleaner. In case we need to pass a specific ABI, it is always possible to do so using a different function name.
        kuduconv does not require you to pass an ABI explicitly anymore, it has a few preloaded ones that will be selected automatically if they match the type being converted.
        implemented the Transaction struct, along with transaction signing.

    kudune enhancements:
        can compile Spring and CDT instead of downloading packages when building an image
        can run on MacOS (tested with Orbstack), will use an amd64 base image
        increased cpu max usage time for transactions to be able to run on lower-power machines

    introduction of the kudu python bindings (in the kudu-py subfolder)
        wraps a few classes for now for pushing transactions to a running node: Name, PermissionLevel, Action, Transaction, SignedTransaction, APIClient, PublicKey, PrivateKey
        has a very basic, very insecure wallet for managing keys (useful for running tests and during dev) (This needs to be moved to a rust struct with python bindings)

There is some cleanup left to be performed but we can already successfully run the kudu-py/test_chain.py test that does the following:

    create a new docker container with a fresh install of nodeos
    start nodeos and bootstrap a fully running Vaulta chain
    create a few users
    create a new token
    distribute some of those tokens to the users
    have those users transfer tokens to each other
