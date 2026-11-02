#!/usr/bin/env bash

set -euo pipefail

PROJECT_DIR=$(cd -- "$(dirname -- "$(readlink -f -- "${BASH_SOURCE[0]}")")/.." && pwd)
SCRIPTS_DIR="$PROJECT_DIR/scripts"

# 宿主组合根：项目配置 -> 通用 Docker 基础设施 -> 通用交付工作流。
# shellcheck source=scripts/lib/common.bash
source "$SCRIPTS_DIR/lib/common.bash"
# shellcheck source=scripts/config/rkflash.bash
source "$SCRIPTS_DIR/config/rkflash.bash"
# shellcheck source=scripts/config/rkflash-delivery.bash
source "$SCRIPTS_DIR/config/rkflash-delivery.bash"
# shellcheck source=scripts/lib/container.bash
source "$SCRIPTS_DIR/lib/container.bash"
# shellcheck source=scripts/lib/delivery.bash
source "$SCRIPTS_DIR/lib/delivery.bash"

project_configure "$PROJECT_DIR"
project_delivery_configure "$PROJECT_DIR"
delivery_dispatch "$@"
