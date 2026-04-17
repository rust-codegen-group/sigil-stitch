#!/usr/bin/env bash
set -euo pipefail

source "./lib/utils.sh"

function deploy() {
    local target=$1
    if [ -z "$target" ]; then
        echo "error: no target"
        return 1
    fi
    log_info "deploying to $target"

}

deploy "$@"
