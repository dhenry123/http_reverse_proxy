#!/usr/bin/env bash
set -e

match="target/release/http_reverse_proxy"
running=$(pgrep -f "${match}" | xargs)
if [ "$1" == "d" ]; then
    if [ "${running}" != "" ]; then
        # shellcheck disable=SC2086
        kill -9 ${running}
        echo "process: ${running} killed"
        exit
    else
        echo "No process is running"
        exit
    fi
fi

pathFinal=$(realpath "${match}")

RUST_LOG="debug" "${pathFinal}" &
