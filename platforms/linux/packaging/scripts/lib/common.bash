# 通用出包基础能力。此文件由入口 source，不是独立命令。

packaging_die() {
    echo "错误：$*" >&2
    exit 1
}

packaging_require_command() {
    local command_name=$1
    command -v "$command_name" >/dev/null 2>&1 || packaging_die "缺少命令 $command_name"
}

packaging_recreate_output_dir() {
    local root_dir=$1
    local output_dir=$2

    packaging_remove_output_dir "$root_dir" "$output_dir"
    mkdir -p -- "$output_dir"
}

packaging_remove_output_dir() {
    local root_dir=$1
    local output_dir=$2
    local canonical_root canonical_output

    packaging_require_command readlink
    canonical_root=$(readlink -f -- "$root_dir")
    canonical_output=$(readlink -m -- "$output_dir")
    [[ "$canonical_output" == "$canonical_root"/* ]] || \
        packaging_die "拒绝清理项目目录之外的产出目录：$canonical_output"

    if [[ -e "$canonical_output" ]]; then
        echo "[产出] 清理目录：$canonical_output"
        rm -rf -- "$canonical_output"
    fi
}

packaging_check_debian_dependencies() {
    local dependency package_status
    local -a missing_packages=()

    packaging_require_command dpkg-query
    for dependency in "$@"; do
        package_status=$(dpkg-query -W -f='${Status}' "$dependency" 2>/dev/null || true)
        if [[ "$package_status" != "install ok installed" ]]; then
            missing_packages+=("$dependency")
        fi
    done
    if ((${#missing_packages[@]} > 0)); then
        if [[ "${CONTAINERIZED_BUILD:-0}" == 1 ]]; then
            packaging_die "Docker 镜像缺少构建依赖：${missing_packages[*]}；请更新 Dockerfile 后重建镜像"
        fi
        packaging_die "缺少构建依赖：${missing_packages[*]}"
    fi
    echo "[环境] 系统构建依赖已就绪"
}

packaging_load_cargo_environment() {
    local cargo_environment
    if [[ -n "${CARGO_HOME:-}" ]]; then
        cargo_environment="$CARGO_HOME/env"
    else
        cargo_environment="$HOME/.cargo/env"
    fi
    if [[ -f "$cargo_environment" ]]; then
        # shellcheck disable=SC1090
        source "$cargo_environment"
    fi
}

packaging_select_rust_toolchain() {
    local candidate candidate_version current_bin newest_version rustup_home
    local -a candidates=()

    RUST_SELECTED_BIN=
    RUST_SELECTED_VERSION=
    if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
        current_bin=$(dirname -- "$(command -v rustc)")
        candidates+=("$current_bin")
    fi

    rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
    for candidate in "$rustup_home"/toolchains/*/bin; do
        [[ -x "$candidate/cargo" && -x "$candidate/rustc" ]] || continue
        candidates+=("$candidate")
    done

    for candidate in "${candidates[@]}"; do
        candidate_version=$("$candidate/rustc" --version 2>/dev/null | \
            sed -n 's/^rustc \([^ ]*\).*/\1/p')
        [[ -n "$candidate_version" ]] || continue
        if [[ -z "$RUST_SELECTED_VERSION" ]]; then
            RUST_SELECTED_BIN=$candidate
            RUST_SELECTED_VERSION=$candidate_version
            continue
        fi
        newest_version=$(printf '%s\n%s\n' "$RUST_SELECTED_VERSION" "$candidate_version" | \
            sort -V | tail -n 1)
        if [[ "$newest_version" == "$candidate_version" ]]; then
            RUST_SELECTED_BIN=$candidate
            RUST_SELECTED_VERSION=$candidate_version
        fi
    done

    [[ -n "$RUST_SELECTED_BIN" ]] || return 1
    export PATH="$RUST_SELECTED_BIN:$PATH"
}

packaging_rust_version_is_supported() {
    local current_version=$1
    local minimum_version=${RUST_MINIMUM_VERSION:-1.88}
    local newest_version

    newest_version=$(printf '%s\n%s\n' "$minimum_version" "$current_version" | \
        sort -V | tail -n 1)
    [[ "$newest_version" == "$current_version" ]]
}

packaging_bootstrap_rust_toolchain() {
    local cargo_home rustup_bin rustup_home temporary_dir installer

    [[ "${RUST_BOOTSTRAP_DISABLED:-0}" != 1 ]] || packaging_die \
        "Rust 工具链缺失或版本过旧，且 RUST_BOOTSTRAP_DISABLED=1 禁止自动安装"

    cargo_home=${CARGO_HOME:-$HOME/.cargo}
    rustup_home=${RUSTUP_HOME:-$HOME/.rustup}
    rustup_bin="$cargo_home/bin/rustup"

    if [[ ! -x "$rustup_bin" ]]; then
        packaging_require_command curl
        temporary_dir=$(mktemp -d /tmp/rust-toolchain-bootstrap.XXXXXX)
        installer="$temporary_dir/rustup-init.sh"
        echo "[环境] 未找到 rustup，正在安装用户级 Rust 工具链"
        curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
            https://sh.rustup.rs --output "$installer"
        CARGO_HOME="$cargo_home" RUSTUP_HOME="$rustup_home" \
            sh "$installer" -y --profile minimal --default-toolchain none --no-modify-path
        rm -f -- "$installer"
        rmdir -- "$temporary_dir"
    fi

    [[ -x "$rustup_bin" ]] || packaging_die "rustup 安装后仍不可用：$rustup_bin"
    echo "[环境] 正在准备 stable Rust、rustfmt 和 Clippy"
    CARGO_HOME="$cargo_home" RUSTUP_HOME="$rustup_home" \
        "$rustup_bin" toolchain install stable \
            --profile minimal --component rustfmt --component clippy
}

packaging_prepare_rust_toolchain() {
    local detected_version=未安装

    packaging_load_cargo_environment
    if packaging_select_rust_toolchain; then
        detected_version=$RUST_SELECTED_VERSION
        if packaging_rust_version_is_supported "$RUST_SELECTED_VERSION"; then
            return
        fi
    fi

    echo "[环境] Rust 工具链需要准备：当前 $detected_version，最低 ${RUST_MINIMUM_VERSION:-1.88}"
    packaging_bootstrap_rust_toolchain
    packaging_load_cargo_environment
    hash -r
    packaging_select_rust_toolchain || packaging_die "自动安装后仍未找到 Rust 工具链"
    packaging_rust_version_is_supported "$RUST_SELECTED_VERSION" || packaging_die \
        "自动安装后的 Rust 版本仍过旧：$RUST_SELECTED_VERSION"
}

packaging_rust_can_compile() (
    local probe_output
    probe_output=$(mktemp /tmp/packaging-rust-probe.XXXXXX)
    trap 'rm -f -- "$probe_output"' EXIT
    printf 'fn main() {}\n' | \
        rustc \
            --crate-name packaging_build_probe \
            --emit metadata \
            -o "$probe_output" \
            - >/dev/null 2>&1
)

packaging_check_rust_toolchain() {
    packaging_prepare_rust_toolchain
    packaging_rust_can_compile || \
        packaging_die "stable Rust 工具链无法编译宿主机目标；请先修复工具链，出包脚本不会修改它"
    echo "[环境] Rust 工具链已就绪：$(rustc --version)"
}

packaging_assert_no_matches() {
    local description=$1
    local pattern=$2
    shift 2
    if rg -n "$pattern" "$@"; then
        packaging_die "架构检查失败：$description"
    fi
}

packaging_check_elf_glibc() {
    local binary_path=$1
    local expected_architecture=$2
    local baseline_glibc=$3
    local supported_targets=$4
    local binary_type highest_index highest_glibc newest_version command_name
    local -a required_glibc_versions

    [[ -x "$binary_path" ]] || packaging_die "缺少待检查的可执行文件 $binary_path"
    for command_name in file grep readelf sort; do
        packaging_require_command "$command_name"
    done

    binary_type=$(file -b "$binary_path")
    if [[ "$expected_architecture" == amd64 ]] && \
       [[ "$binary_type" != *"ELF 64-bit"* || "$binary_type" != *"x86-64"* ]]; then
        packaging_die "发布程序不是 amd64 ELF：$binary_type"
    fi

    mapfile -t required_glibc_versions < <(
        readelf --version-info "$binary_path" | \
            grep -o 'GLIBC_[0-9][0-9.]*' | \
            sort -Vu
    )
    if ((${#required_glibc_versions[@]} == 0)); then
        if [[ "$binary_type" == *"statically linked"* ]]; then
            echo "Linux 兼容性基线检查通过"
            echo "  架构：$expected_architecture"
            echo "  链接方式：静态链接（不依赖目标机 glibc 符号）"
            echo "  支持目标：$supported_targets"
            return
        fi
        packaging_die "无法确定程序所需的 glibc 符号版本"
    fi

    highest_index=$((${#required_glibc_versions[@]} - 1))
    highest_glibc=${required_glibc_versions[$highest_index]#GLIBC_}
    newest_version=$(printf '%s\n%s\n' "$baseline_glibc" "$highest_glibc" | sort -V | tail -n 1)
    [[ "$newest_version" == "$baseline_glibc" ]] || \
        packaging_die "程序需要 glibc $highest_glibc，超过兼容基线 $baseline_glibc"

    echo "Linux 兼容性基线检查通过"
    echo "  架构：$expected_architecture"
    echo "  最高 glibc 符号版本：$highest_glibc"
    echo "  支持目标：$supported_targets"
}
