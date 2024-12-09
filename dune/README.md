
# TODO / FIXME

- better error handling instead of unwrap, esp. on docker commands
  -> bubble up errors and display then with color_eyre?

- check experimental packages for leap (arm64) (see: bootstrap_leap.sh)

- update from Leap to Spring

- check some use cases from here: <https://docs.eosnetwork.com/docs/latest/node-operation/api-node/>
  can we fulfill them?

- embed deploy_eos_image.py in the binary so we can run "dune build-image" from anywhere

- use IndexMap and the indexmap feature on configparser to ensure we do not mess up the config file order
  also ensure we're keeping comments from the config file

- make sure we can generate a decent documentation

- try to follow guidelines from <https://clig.dev>


# README

Here is a list of workflows that should be enabled by the `dune` utility
and how they can be performed. This should serve both as design document
and end-to-end testing of the binary to assess its usefulness / ease-of-use

By default, we run nodeos in a docker container with the name `eos_container`

## build a base image to be used for creating containers with all the tools installed

```sh
dune build-image                # use a default Ubuntu base image
dune build-image wackou:devbox  # use a provided image
```


## build a new container from scratch

```sh
dune destroy    # ensure that we don't have a lingering docker container
dune start-node
```

## build a new container with a given config file

```sh
dune destroy    # ensure that we don't have a lingering docker container
dune start-node --config <CONFIG_FILE.INI>
```

## set our own default config instead of nodeos default

in particular, we want to expose the http port to all listeners, not only localhost

```sh
dune set-config default
```

you can set specific values like so:
```sh
dune set-config http-server-address=0.0.0.0:8888 chain-state-db-size-mb=65536 contracts-console=true
```
