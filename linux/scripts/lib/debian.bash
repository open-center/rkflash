# 通用 Debian 与离线依赖出包流程。项目差异全部通过配置和钩子注入。

debian_read_runtime_dependencies() {
    sed -e 's/[[:space:]]*#.*$//' -e '/^[[:space:]]*$/d' \
        "$RUNTIME_DEPENDENCIES_FILE"
}

debian_prepare_system_dependencies() {
    local dependency group package_status strict_packages=0
    local -a command_prefix=()
    local -a missing_packages=()
    local -a required_packages=()
    local -A seen_packages=()

    (($# > 0)) || packaging_die "未指定系统依赖组"
    declare -F project_select_dependency_group >/dev/null || \
        packaging_die "缺少项目依赖组钩子"
    for group in "$@"; do
        project_select_dependency_group "$group"
        required_packages+=("${PROJECT_DEPENDENCY_PACKAGES[@]}")
        if [[ "$PROJECT_DEPENDENCY_STRICT" == 1 ]]; then
            strict_packages=1
        fi
    done

    packaging_require_command apt-get
    packaging_require_command dpkg-query
    for dependency in "${required_packages[@]}"; do
        [[ -z "${seen_packages[$dependency]:-}" ]] || continue
        seen_packages[$dependency]=1
        if ((strict_packages == 0)) && command -v "$dependency" >/dev/null 2>&1; then
            continue
        fi
        package_status=$(dpkg-query -W -f='${Status}' "$dependency" 2>/dev/null || true)
        if [[ "$package_status" != "install ok installed" ]]; then
            missing_packages+=("$dependency")
        fi
    done
    ((${#missing_packages[@]} > 0)) || return 0

    [[ "${SYSTEM_BOOTSTRAP_DISABLED:-0}" != 1 ]] || packaging_die \
        "缺少系统依赖：${missing_packages[*]}；SYSTEM_BOOTSTRAP_DISABLED=1 禁止自动安装"
    if ((EUID != 0)); then
        packaging_require_command sudo
        command_prefix=(sudo)
    fi

    echo "[环境] 正在安装系统构建依赖：${missing_packages[*]}"
    "${command_prefix[@]}" apt-get update
    "${command_prefix[@]}" apt-get install -y --no-install-recommends \
        "${missing_packages[@]}"
    for dependency in "${missing_packages[@]}"; do
        package_status=$(dpkg-query -W -f='${Status}' "$dependency" 2>/dev/null || true)
        [[ "$package_status" == "install ok installed" ]] || packaging_die \
            "系统依赖安装后仍不可用：$dependency"
    done
}

debian_write_metadata() {
    local stage_dir=$1 metadata_architecture=$2 installed_size=$3 runtime_depends=$4

    mkdir -p "$stage_dir/DEBIAN"
    cat >"$stage_dir/DEBIAN/control" <<EOF
Package: $PACKAGE_NAME
Version: $PACKAGE_VERSION
Section: $PACKAGE_SECTION
Priority: optional
Architecture: $metadata_architecture
Installed-Size: $installed_size
Depends: $runtime_depends
Conflicts: $PACKAGE_CONFLICTS
Replaces: $PACKAGE_REPLACES
Provides: $PACKAGE_PROVIDES
Maintainer: $PACKAGE_MAINTAINER
Description: $PACKAGE_DESCRIPTION
$PACKAGE_DESCRIPTION_DETAIL
EOF
    if declare -F project_stage_maintainer_scripts >/dev/null; then
        project_stage_maintainer_scripts "$stage_dir"
    fi
}

debian_write_offline_readme() {
    local bundle_dir=$1 distribution=$2 distribution_version=$3
    local distribution_codename=$4 architecture=$5

    cat >"$bundle_dir/README.txt" <<EOF
$PACKAGE_NAME $PACKAGE_VERSION 离线安装包

适用系统：$distribution $distribution_version ($distribution_codename) / $architecture

校验：sha256sum --check SHA256SUMS
安装：./install.sh
卸载：sudo apt purge $PACKAGE_NAME

所有应用和依赖均来自包内 .deb 组成的本地 file: 仓库，软件源已隔离，不会联网下载。
目标机上比包内更新的版本会被保留，不会静默降级。
APT 要求当前 dpkg 状态完整；若存在其他半安装软件包会立即报错，不会联网修复。
卸载命令不读取或更新任何软件仓库。
EOF
}

debian_write_offline_installer() {
    local bundle_dir=$1

    cat >"$bundle_dir/install.sh" <<EOF
#!/bin/sh
set -eu

package_name='$PACKAGE_NAME'
script_dir=\$(CDPATH='' cd -- "\$(dirname -- "\$0")" && pwd -P)
staging_dir=

if [ "\$(id -u)" -eq 0 ]; then
    as_root() { "\$@"; }
else
    command -v sudo >/dev/null 2>&1 || {
        echo '错误：安装需要 root 权限，且当前系统缺少 sudo' >&2
        exit 1
    }
    as_root() { sudo "\$@"; }
fi

cleanup() {
    case "\$staging_dir" in
        /var/tmp/\${package_name}-offline-install.*)
            cleanup_target=\$staging_dir
            staging_dir=
            as_root rm -rf -- "\$cleanup_target"
            ;;
    esac
    for temp_file in "\${packages_index:-}" "\${offline_list:-}"; do
        [ -n "\$temp_file" ] && rm -f -- "\$temp_file"
    done
}
trap cleanup EXIT

staging_dir=\$(as_root mktemp -d "/var/tmp/\${package_name}-offline-install.XXXXXX")
case "\$staging_dir" in
    /var/tmp/\${package_name}-offline-install.*) ;;
    *)
        echo "错误：安装暂存目录不符合预期：\$staging_dir" >&2
        exit 1
        ;;
esac
as_root chmod 0755 "\$staging_dir"
as_root cp -- "\$script_dir"/packages/*.deb "\$staging_dir/"

# 把暂存目录组织成本地 file: 仓库，让 apt 依据仓库索引自行安排安装
# 顺序。直接在命令行传 .deb 文件有两个缺陷：升级已装包时会触发 apt
# 的 "Pathname to install is not absolute" 内部错误，且无法表达
# "保留目标机上比包内更新的版本"。仓库方式两者都能正确处理。
# 索引必须包含完整 control 字段（Depends 等）；缺了它们 apt 会把
# 所有包当作零依赖，只安装命令行点名的那个包。
packages_index=\$(mktemp "/tmp/\${package_name}-packages-index.XXXXXX")
offline_list=\$(mktemp "/tmp/\${package_name}-offline-list.XXXXXX")
(
    cd "\$staging_dir" || exit 1
    for package_file in *.deb; do
        dpkg-deb -f "\$package_file"
        printf 'Filename: %s\nSize: %s\nSHA256: %s\n\n' \\
            "\$package_file" "\$(stat -c %s "\$package_file")" \\
            "\$(sha256sum "\$package_file" | cut -d ' ' -f1)"
    done
) >"\$packages_index"
# 必须显式 0644：mktemp 源文件是 0600，cp 会带过去，_apt 读取索引会被拒。
as_root install -m 0644 "\$packages_index" "\$staging_dir/Packages"

# 独立源列表：除本地仓库外没有任何软件源，天然禁止联网下载；
# 独立索引目录：不触碰目标机自己的 APT 索引。
printf 'deb [trusted=yes] file://%s ./\n' "\$staging_dir" >"\$offline_list"
apt_options="-o Dir::Etc::sourcelist=\$offline_list -o Dir::Etc::sourceparts=/dev/null"
apt_options="\$apt_options -o Dir::State::lists=\$staging_dir/apt-lists"
# apt 以 _apt 沙箱用户写索引，目录需放开写权限。
as_root mkdir -p "\$staging_dir/apt-lists/partial"
as_root chmod 0777 "\$staging_dir/apt-lists" "\$staging_dir/apt-lists/partial"
as_root apt-get \$apt_options update
as_root apt-get \$apt_options --no-install-recommends install -y "\$package_name"
EOF
    chmod 0755 "$bundle_dir/install.sh"
}

debian_build_package_configured() (
    local detected_architecture stage_dir installed_size runtime_depends package_file command_name
    local installer_file
    local distribution distribution_version

    if [[ -n "${DELIVERY_BASELINE_VERSION:-}" ]]; then
        # shellcheck disable=SC1091
        source /etc/os-release
        distribution=${ID:-}
        distribution_version=${VERSION_ID:-}
        [[ "$distribution" == "${DELIVERY_BASELINE_DISTRIBUTION:-}" && \
           "$distribution_version" == "$DELIVERY_BASELINE_VERSION" ]] || \
            packaging_die ".deb 必须在 ${DELIVERY_BASELINE_DISTRIBUTION:-未知发行版} $DELIVERY_BASELINE_VERSION 基线镜像中构建；实际为 $distribution $distribution_version"
    fi

    packaging_check_debian_dependencies "${BUILD_DEPENDENCIES[@]}"
    detected_architecture=$(dpkg --print-architecture)
    [[ "$detected_architecture" == "$PACKAGE_ARCHITECTURE" ]] || \
        packaging_die "当前项目包仅支持 $PACKAGE_ARCHITECTURE，检测到 $detected_architecture"
    packaging_check_rust_toolchain
    for command_name in dpkg-deb desktop-file-validate install rg sha256sum strip; do
        packaging_require_command "$command_name"
    done

    mkdir -p "$DIST_DIR"
    project_validate_sources
    project_validate_package_inputs

    echo "[1/4] 编译 Rust release 版本"
    cargo build --manifest-path "$CARGO_MANIFEST" --release --locked
    project_validate_binary

    stage_dir=$(mktemp -d "/tmp/${PACKAGE_NAME}-deb.XXXXXX")
    # mktemp 生成 0700 权限，dpkg-deb 会把根目录权限写进包内，统一放宽为 0755。
    chmod 0755 "$stage_dir"
    trap 'rm -rf -- "$stage_dir"' EXIT
    echo "[2/4] 生成 Debian 文件树"
    project_stage_payload "$stage_dir"

    installed_size=$(du -sk "$stage_dir/usr" | cut -f1)
    runtime_depends=$(debian_read_runtime_dependencies | paste -sd, - | sed 's/,/, /g')
    debian_write_metadata \
        "$stage_dir" "$detected_architecture" "$installed_size" "$runtime_depends"

    echo "[3/4] 校验 Debian 元数据"
    package_file="$DIST_DIR/${PACKAGE_NAME}_${PACKAGE_VERSION}_${detected_architecture}.deb"
    echo "[4/4] 构建 $package_file"
    dpkg-deb --root-owner-group --build "$stage_dir" "$package_file"
    installer_file="$DIST_DIR/install.sh"
    install -m 0755 "$DEBIAN_INSTALLER_TEMPLATE" "$installer_file"
    echo "完成：$package_file"
    echo "安装：$installer_file"
    sha256sum "$package_file"
)

debian_build_package() (
    local root_dir=$1

    project_configure "$root_dir"
    packaging_recreate_output_dir "$root_dir" "$DIST_DIR"
    debian_build_package_configured
)

debian_build_offline_bundle_from_package() (
    local root_dir=$1
    local distribution distribution_version distribution_codename architecture
    local app_package stage_root bundle_name bundle_dir packages_dir
    local empty_status solver_output download_plan package_mirror
    local quoted_url url filename expected_size checksum expected_hash destination temporary_download
    local download_index=0 archive temporary_archive command_name dependency checksum_manifest
    local -a runtime_dependencies runtime_packages resolved_packages

    project_configure "$root_dir"
    for command_name in apt-get curl dpkg dpkg-deb sha256sum sha512sum stat tar; do
        packaging_require_command "$command_name"
    done
    # shellcheck disable=SC1091
    source /etc/os-release
    distribution=${ID:-}
    distribution_version=${VERSION_ID:-}
    distribution_codename=${VERSION_CODENAME:-}
    architecture=$(dpkg --print-architecture)
    case "$distribution" in ubuntu|debian) ;; *) packaging_die "仅支持 Ubuntu/Debian 离线包" ;; esac
    [[ -n "$distribution_version" && -n "$distribution_codename" ]] || \
        packaging_die "无法确定系统版本"
    if [[ -n "${OFFLINE_TARGET_VERSION:-}" ]]; then
        [[ "$distribution" == "${OFFLINE_TARGET_DISTRIBUTION:-}" ]] || packaging_die \
            "离线目标要求 ${OFFLINE_TARGET_DISTRIBUTION:-未知发行版} $OFFLINE_TARGET_VERSION，实际镜像为 $distribution"
        [[ "$distribution_version" == "$OFFLINE_TARGET_VERSION" ]] || packaging_die \
            "离线目标要求 $distribution $OFFLINE_TARGET_VERSION，实际镜像为 $distribution_version"
    fi
    [[ "$architecture" == "$PACKAGE_ARCHITECTURE" ]] || \
        packaging_die "当前项目包仅支持 $PACKAGE_ARCHITECTURE，检测到 $architecture"

    app_package="$DIST_DIR/${PACKAGE_NAME}_${PACKAGE_VERSION}_${architecture}.deb"
    [[ -f "$app_package" ]] || packaging_die "缺少应用安装包 $app_package"
    [[ $(dpkg-deb -f "$app_package" Package) == "$PACKAGE_NAME" ]] || \
        packaging_die "应用安装包名称不匹配：$app_package"
    [[ $(dpkg-deb -f "$app_package" Version) == "$PACKAGE_VERSION" ]] || \
        packaging_die "应用安装包版本不匹配：$app_package"
    [[ $(dpkg-deb -f "$app_package" Architecture) == "$architecture" ]] || \
        packaging_die "应用安装包架构不匹配：$app_package"
    mapfile -t runtime_dependencies < <(debian_read_runtime_dependencies)
    runtime_packages=()
    for dependency in "${runtime_dependencies[@]}"; do
        runtime_packages+=("${dependency%% *}")
    done

    stage_root=$(mktemp -d "/tmp/${PACKAGE_NAME}-offline.XXXXXX")
    trap 'rm -rf -- "$stage_root"' EXIT
    bundle_name="${PACKAGE_NAME}_${PACKAGE_VERSION}_${distribution}-${distribution_version}_${architecture}_offline"
    bundle_dir="$stage_root/$bundle_name"
    packages_dir="$bundle_dir/packages"
    mkdir -p "$packages_dir"
    solver_status="$stage_root/solver-dpkg-status"
    solver_output="$stage_root/dependency-plan"
    download_plan="$stage_root/download-plan"
    # 优先用干净基线系统的 dpkg 状态求解闭包，只打包基线机器缺少的包；
    # 空状态闭包会把全部 Essential 包一并拉进来，包体膨胀且无意义。
    if [[ -n "${BASELINE_DPKG_STATUS:-}" && -f "$BASELINE_DPKG_STATUS" ]]; then
        cp "$BASELINE_DPKG_STATUS" "$solver_status"
        closure_basis="基线系统"
    else
        : >"$solver_status"
        closure_basis="空状态（将包含全部基础包）"
    fi

    echo "[1/4] 解析干净系统上的完整运行依赖闭包（$closure_basis）"
    apt-get -o "Dir::State::status=$solver_status" \
        -o "APT::Architecture=$architecture" \
        -o "APT::Architectures::=$architecture" \
        -o Debug::NoLocking=1 --no-install-recommends --simulate \
        install "${runtime_packages[@]}" >"$solver_output"
    # apt 模拟输出有两种 Inst 行：新装 `Inst 包 (版本 ...)` 和升级
    # `Inst 包 [旧版本] (新版本 ...)`。可选的 [旧版本] 段必须跳过，
    # 否则基线状态下需要升级的包会被漏出闭包，导致离线安装时依赖断裂。
    mapfile -t resolved_packages < <(
        sed -n -E 's/^Inst ([^ ]+) (\[[^]]*\] )?\(([^ )]+).*/\1=\3/p' "$solver_output" |
            sort -u
    )
    if ((${#resolved_packages[@]} == 0)); then
        echo "[2/4] 基线系统已满足全部运行依赖，无需下载依赖包"
    else
        apt-get -o "APT::Architecture=$architecture" \
            -o "APT::Architectures::=$architecture" --print-uris \
            download "${resolved_packages[@]}" >"$download_plan"
        if [[ -n "${OFFLINE_APT_MIRROR:-}" ]]; then
            package_mirror=$OFFLINE_APT_MIRROR
        elif declare -F project_default_package_mirror >/dev/null; then
            package_mirror=$(project_default_package_mirror "$distribution")
        else
            package_mirror=
        fi

        echo "[2/4] 取得 ${#resolved_packages[@]} 个依赖包并校验 SHA-512"
        while read -r quoted_url filename expected_size checksum; do
            url=${quoted_url#\'}
            url=${url%\'}
            expected_hash=${checksum#SHA512:}
            destination="$packages_dir/$filename"
            download_index=$((download_index + 1))
            if [[ -n "$package_mirror" && "$url" == */pool/* ]]; then
                url="${package_mirror%/}/pool/${url#*/pool/}"
            fi
            temporary_download="$packages_dir/.${filename}.part"
            echo "[$download_index/${#resolved_packages[@]}] 下载：$filename"
            curl --fail --location --show-error --silent \
                --connect-timeout 15 --speed-limit 1024 --speed-time 30 \
                --retry 3 --retry-delay 2 --retry-all-errors \
                --output "$temporary_download" "$url"
            [[ $(stat -c '%s' "$temporary_download") == "$expected_size" ]] || \
                packaging_die "$filename 文件大小不符"
            printf '%s  %s\n' "$expected_hash" "$temporary_download" | \
                sha512sum --check --quiet - || packaging_die "$filename SHA-512 校验失败"
            mv -f "$temporary_download" "$destination"
        done <"$download_plan"
        ((download_index == ${#resolved_packages[@]})) || packaging_die "APT 下载计划数量不完整"
    fi
    cp "$app_package" "$packages_dir/"

    cat >"$bundle_dir/BUILD-INFO" <<EOF
package=$PACKAGE_NAME
version=$PACKAGE_VERSION
distribution=$distribution
distribution_version=$distribution_version
codename=$distribution_codename
architecture=$architecture
dependency_packages=${#resolved_packages[@]}
EOF
    debian_write_offline_readme \
        "$bundle_dir" "$distribution" "$distribution_version" "$distribution_codename" "$architecture"
    debian_write_offline_installer "$bundle_dir"

    echo "[3/4] 生成全包 SHA-256 校验清单"
    checksum_manifest="$stage_root/SHA256SUMS"
    (
        cd "$bundle_dir" || exit 1
        find . -type f ! -name SHA256SUMS -print0 | sort -z | \
            xargs -0 sha256sum
    ) >"$checksum_manifest"
    mv -f "$checksum_manifest" "$bundle_dir/SHA256SUMS"
    archive="$DIST_DIR/${bundle_name}.tar.gz"
    temporary_archive="$stage_root/${bundle_name}.tar.gz"
    echo "[4/4] 生成 $archive"
    tar -C "$stage_root" -czf "$temporary_archive" "$bundle_name"
    mv -f "$temporary_archive" "$archive"
    (
        cd "$DIST_DIR" || exit 1
        sha256sum "$(basename "$archive")" >"$(basename "$archive").sha256"
    )
    echo "完成：$archive"
    du -h "$archive"
    cat "$archive.sha256"
)
