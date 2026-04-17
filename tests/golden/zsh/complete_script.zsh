#!/usr/bin/env zsh
setopt ERR_EXIT PIPE_FAIL

source "./lib/utils.zsh"

function deploy() {
    local target=$1
    if [[ -z "$target" ]]; then
        echo "error: no target"
        return 1
    fi
    log_info "deploying to $target"

}

deploy "$@"
