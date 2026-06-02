<!--
SPDX-FileCopyrightText: 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# TODO / FIXME

- review the configuration of the network ports in the kudune CLI, it seems something is not properly
  matched between all the ports, the http_addr in the config.ini file, the one in the Dune instance, etc.

- better error handling instead of unwrap, esp. on docker commands
  -> bubble up errors and display then with color_eyre?

- check some use cases from here: <https://docs.vaulta.com/docs/latest/node-operation/api-node>
  can we fulfill them?

- use IndexMap on the node config to ensure we do not mess up the config file order
  also ensure we're keeping comments from the config file when round-tripping it

- try to replace all calls to cleos with direct call to nodes

- make sure we can generate a decent documentation

- try to follow guidelines from <https://clig.dev>
