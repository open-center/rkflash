#!/usr/bin/env bash

set -euo pipefail

PACKAGING_DIR=$(cd -- "$(dirname -- "$(readlink -f -- "${BASH_SOURCE[0]}")")" && pwd)
PROJECT_DIR=$(cd -- "$PACKAGING_DIR/../../.." && pwd)
SCRIPTS_DIR="$PACKAGING_DIR/scripts"

# 宿主机入口只负责进入固定的 Docker 构建环境。容器中的同一脚本继续负责命令分发，
# 从而保证所有 Rust、系统库和 Debian 打包工具都来自镜像而不是宿主 Ubuntu。
if [[ "${CONTAINERIZED_BUILD:-0}" != 1 || ! -f /.dockerenv ]]; then
    exec "$SCRIPTS_DIR/docker.bash" "$@"
fi

# 单一组合入口：这里只装配命令，具体实现位于 scripts/lib 与 scripts/config。
# shellcheck source=scripts/lib/common.bash
source "$SCRIPTS_DIR/lib/common.bash"
# shellcheck source=scripts/config/rkflash.bash
source "$SCRIPTS_DIR/config/rkflash.bash"
# shellcheck source=scripts/config/rkflash-delivery.bash
source "$SCRIPTS_DIR/config/rkflash-delivery.bash"
# shellcheck source=scripts/lib/rust.bash
source "$SCRIPTS_DIR/lib/rust.bash"
# shellcheck source=scripts/lib/debian.bash
source "$SCRIPTS_DIR/lib/debian.bash"

[[ -f "$PROJECT_DIR/Cargo.toml" ]] || packaging_die "必须在 rkflash 源码目录中执行"
cd "$PROJECT_DIR"
project_configure "$PROJECT_DIR"
project_delivery_configure "$PROJECT_DIR"

usage() {
    cat <<EOF
用法：
  ./build.sh [命令]

开发命令：
  release       只构建整个 Rust 工作区的 release 二进制
  setup         拉取基础镜像并准备完整 Docker 构建环境
  toolchain     不使用缓存重建镜像，更新其中的 stable Rust 工具链
  run [参数...] 运行桌面程序，额外参数原样传给程序
  fmt           检查 Rust 格式
  test          运行工作区全部测试
  lint          运行 Clippy，并把警告视为错误
  architecture  检查前后端、领域、应用和基础设施边界
  check         依次执行 fmt、architecture、test、lint
  clean         清理统一产出目录 output/（不依赖 Rust/Cargo）

交付命令：
  build         构建 release 程序和通用 Debian 安装包（默认）
  package       build 的等价别名
  install       安全暂存并在当前 Ubuntu 安装已生成的 .deb
  offline       复用基线 .deb，为指定 Ubuntu 版本生成完整离线安装包
  offline-all   复用同一个基线 .deb，生成 $DELIVERY_DISTRIBUTION_DISPLAY ${DELIVERY_TARGET_VERSIONS[*]} 离线包

宿主机只需要 Docker。编译中间文件统一写入 output/target，交付物写入 output/dist。
build/package/offline/offline-all 会先安全清理 output/dist/；install 不重新构建。
EOF
}

command_name=${1:-build}
if (($# > 0)); then
    shift
fi

case "$command_name" in
    release)
        (($# == 0)) || packaging_die "release 不接受其他参数"
        rust_build_release
        ;;
    setup)
        (($# == 0)) || packaging_die "setup 不接受其他参数"
        debian_prepare_system_dependencies development architecture
        rust_update_toolchain
        ;;
    toolchain)
        (($# == 0)) || packaging_die "toolchain 不接受其他参数"
        rust_update_toolchain
        ;;
    run) rust_run "$@" ;;
    fmt)
        (($# == 0)) || packaging_die "fmt 不接受其他参数"
        rust_format
        ;;
    test)
        (($# == 0)) || packaging_die "test 不接受其他参数"
        rust_test
        ;;
    lint)
        (($# == 0)) || packaging_die "lint 不接受其他参数"
        rust_lint
        ;;
    architecture)
        (($# == 0)) || packaging_die "architecture 不接受其他参数"
        rust_check_architecture
        ;;
    check)
        (($# == 0)) || packaging_die "check 不接受其他参数"
        rust_check
        ;;
    clean)
        (($# == 0)) || packaging_die "clean 不接受其他参数"
        rust_clean
        ;;
    build|package)
        (($# == 0)) || packaging_die "$command_name 不接受其他参数"
        debian_prepare_system_dependencies packaging
        debian_build_package "$PROJECT_DIR"
        ;;
    offline)
        packaging_die "offline 必须由宿主 Docker 编排入口执行"
        ;;
    offline-bundle)
        (($# == 0)) || packaging_die "offline-bundle 不接受其他参数"
        debian_prepare_system_dependencies packaging
        debian_build_offline_bundle_from_package "$PROJECT_DIR"
        ;;
    offline-all)
        packaging_die "offline-all 必须由宿主 Docker 编排入口执行"
        ;;
    help|-h|--help)
        (($# == 0)) || packaging_die "help 不接受其他参数"
        usage
        ;;
    *)
        usage >&2
        packaging_die "未知命令 $command_name"
        ;;
esac
