# README

Here is a list of workflows that should be enabled by the `kudune` utility
and how they can be performed. This should serve both as design document
and end-to-end testing of the binary to assess its usefulness / ease-of-use.

By default, we run nodeos in a docker container with the name `vaulta_nodeos`
(can be changed with the `--image` option).


## Show list of available commands

```sh
kudune
```


## Build a base image to be used for creating containers with all the tools installed

```sh
kudune build-image                # use a default Ubuntu base image
kudune build-image wackou:devbox  # use a provided base image

kudune build-image --compile      # compile Spring and CDT instead of using precompiled packages

# specify versions of components to be installed
kudune build-image --spring 1.2.2 --cdt 4.1.1 --system-contracts 3.10.0

kudune -vv build-image --compile  # show detailed info of what's going on
```


## Build a new container from scratch and run nodeos

```sh
kudune destroy    # ensure that we don't have a lingering docker container
kudune start-node
```


## Build a new container with a given config file

```sh
kudune destroy    # ensure that we don't have a lingering docker container
kudune start-node --config <CONFIG_FILE.INI>
```


## Set our own default config instead of nodeos default

in particular, we want to expose the http port to all listeners, not only localhost

```sh
kudune set-config default
```

you can set specific values like so:
```sh
kudune set-config http-server-address=0.0.0.0:8888 chain-state-db-size-mb=65536 contracts-console=true
```
