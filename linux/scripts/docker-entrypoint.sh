#!/usr/bin/env bash

set -euo pipefail

[[ -d /workspace ]] || {
    echo "错误：容器内缺少 /workspace 源码挂载" >&2
    exit 1
}

# The container runs with the invoking user's uid/gid, so all generated files
# remain editable on the host. Writable caches therefore live in the bind mount.
mkdir -p -- "${HOME:?}" "${CARGO_HOME:?}" /workspace/output/target

exec /workspace/build.sh "$@"

