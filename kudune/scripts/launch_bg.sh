#!/bin/bash

# SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
# SPDX-License-Identifier: AGPL-3.0-or-later

"$@" >/app/nodeos.log 2>&1 &
