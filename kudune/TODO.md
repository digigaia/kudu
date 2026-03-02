# TODO / FIXME

- add more actions from DUNES, like `get-table`, `version-all`, `upgrade` (maybe?)

- review the configuration of the network ports in the kudune CLI, it seems something is not properly
  matched between all the ports, the http_addr in the config.ini file, the one in the Dune instance, etc.

- check with <https://github.com/AntelopeIO/spring/blob/main/tutorials/bios-boot-tutorial/bios-boot-tutorial.py>
  that `kudune bootstrap` is correct and complete. Pay special attention to the vaulta transition.

- better error handling instead of unwrap, esp. on docker commands
  -> bubble up errors and display then with color_eyre?

- check some use cases from here: <https://docs.vaulta.com/docs/latest/node-operation/api-node>
  can we fulfill them?

- use IndexMap on the node config to ensure we do not mess up the config file order
  also ensure we're keeping comments from the config file when round-tripping it

- make sure we can generate a decent documentation

- try to follow guidelines from <https://clig.dev>

- try to optimize docker image using `dive` or `xray-tui` to check it
