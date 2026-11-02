# rkflash/Rockchip 特殊配置与钩子。通用出包流程不得直接感知这些细节。
# shellcheck disable=SC2034

project_configure() {
    local root_dir=$1

    PROJECT_DIR=$root_dir
    PACKAGE_NAME=rkflash
    APPLICATION_PACKAGE=rkflash
    RUST_MINIMUM_VERSION=1.88
    # 只从 [workspace.package] 段取版本，避免文件前面出现其他 version 字段时误读。
    PACKAGE_VERSION=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' \
        "$PROJECT_DIR/Cargo.toml" | head -n 1)
    PACKAGE_ARCHITECTURE=amd64
    PACKAGE_SECTION=devel
    PACKAGE_MAINTAINER='rkflash contributors <noreply@example.invalid>'
    PACKAGE_DESCRIPTION='Rockchip USB flashing desktop utility'
    PACKAGE_DESCRIPTION_DETAIL=' A Linux desktop frontend compatible with Rockchip upgrade_tool workflows,
 including image download, firmware upgrade, device inspection and LBA tools.'
    # rkflash 由 rktools 改名而来：安装时自动替换旧包，避免两个版本并存。
    PACKAGE_CONFLICTS='rk-flash-studio, rktools'
    PACKAGE_REPLACES='rk-flash-studio, rktools'
    # 带版本的 Provides 才能满足依赖方对 rk-flash-studio (>= x) 这类带版本约束。
    PACKAGE_PROVIDES="rk-flash-studio (= $PACKAGE_VERSION)"
    GLIBC_BASELINE=2.35
    CARGO_MANIFEST="$PROJECT_DIR/Cargo.toml"
    OUTPUT_ROOT="$PROJECT_DIR/output"
    # 正式出包由宿主编排器传入独立目录，禁止复用较新 Ubuntu 的链接产物。
    CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-"$OUTPUT_ROOT/target"}
    RELEASE_BINARY="$CARGO_TARGET_DIR/release/$PACKAGE_NAME"
    DIST_DIR="$OUTPUT_ROOT/dist"
    DEBIAN_INSTALLER_TEMPLATE="$PROJECT_DIR/scripts/install-deb.bash"
    export CARGO_TARGET_DIR
    RUNTIME_DEPENDENCIES_FILE="$PROJECT_DIR/packaging/runtime-dependencies.txt"
    UPGRADE_TOOL="$PROJECT_DIR/tools/upgrade_tool"
    BOOTSTRAP_DEPENDENCIES=(ca-certificates curl)
    DEVELOPMENT_DEPENDENCIES=(
        build-essential ca-certificates curl libgl1-mesa-dev libwayland-dev
        libx11-dev libxi-dev libxkbcommon-dev pkg-config
    )
    ARCHITECTURE_DEPENDENCIES=(ripgrep)
    BUILD_DEPENDENCIES=(
        binutils build-essential ca-certificates coreutils curl desktop-file-utils dpkg file libgl1-mesa-dev
        libudev-dev libusb-1.0-0-dev libwayland-dev libx11-dev libxi-dev
        libxkbcommon-dev pkg-config ripgrep
    )

    [[ -n "$PACKAGE_VERSION" ]] || packaging_die "无法从 Cargo.toml 读取版本号"
}

project_select_dependency_group() {
    local group=$1

    PROJECT_DEPENDENCY_PACKAGES=()
    PROJECT_DEPENDENCY_STRICT=0
    case "$group" in
        bootstrap) PROJECT_DEPENDENCY_PACKAGES=("${BOOTSTRAP_DEPENDENCIES[@]}") ;;
        development) PROJECT_DEPENDENCY_PACKAGES=("${DEVELOPMENT_DEPENDENCIES[@]}") ;;
        architecture) PROJECT_DEPENDENCY_PACKAGES=("${ARCHITECTURE_DEPENDENCIES[@]}") ;;
        packaging)
            PROJECT_DEPENDENCY_PACKAGES=("${BUILD_DEPENDENCIES[@]}")
            PROJECT_DEPENDENCY_STRICT=1
            ;;
        *) packaging_die "未知系统依赖组：$group" ;;
    esac
}

project_validate_sources() {
    packaging_require_command rg
    (
        cd "$PROJECT_DIR" || packaging_die "无法进入项目目录：$PROJECT_DIR"
        if rg -ni \
            'rkflash|rockchip|upgrade_tool|rk-flash-studio|71-rkflash|RKFLASH_|22\.04|24\.04|26\.04' \
            scripts/lib; then
            packaging_die "通用脚本层包含项目名、平台特例或固定发行版版本"
        fi
        if rg -n 'apt-get|dpkg-deb|cargo build|upgrade_tool' build.sh; then
            packaging_die "出包入口包含了业务实现，应只保留装配和参数分发"
        fi
        packaging_assert_no_matches \
            "契约、配置或后端 crate 依赖了 GUI 框架" \
            'eframe|egui|rfd' \
            app/api/src app/config/src app/backend/src
        packaging_assert_no_matches \
            "进程执行逻辑越过基础设施边界" \
            'std::process|Command::new' \
            app/api/src app/config/src app/backend/src/domain \
            app/backend/src/application app/frontend/src
        packaging_assert_no_matches \
            "契约、配置、应用层或领域层直接访问了文件系统" \
            'std::fs|fs::(read|write|metadata|read_dir|File|OpenOptions)' \
            app/api/src app/config/src app/backend/src/domain app/backend/src/application
        packaging_assert_no_matches \
            "前端业务动作绕过 API 能力端口访问了文件系统" \
            'std::fs|fs::(read|write|metadata|read_dir)|\.is_file\(' \
            app/frontend/src/desktop/actions.rs \
            app/frontend/src/desktop/pages \
            app/frontend/src/desktop/view_model.rs
        packaging_assert_no_matches \
            "应用层或领域层反向依赖了基础设施" \
            'crate::infrastructure|ProcessRuntime|UpgradeToolAdapter|JsonConfigurationRepository|LocalFileSystem' \
            app/backend/src/domain app/backend/src/application
        packaging_assert_no_matches \
            "通用进程层包含了 Rockchip 厂商细节" \
            'rockchip|upgrade_tool|\b(LD|DB|DI|WL|UF|EF|RID|RFI|RCI|RCB|PL|RSN|RSM|TD|RD|SSD|SN|RP|UL|RL|EL|RCL)\b' \
            app/backend/src/infrastructure/process
        packaging_assert_no_matches \
            "前端直接依赖了后端基础设施适配器" \
            'ProcessRuntime|UpgradeToolAdapter|JsonConfigurationRepository|LocalFileSystem' \
            app/frontend/src
        packaging_assert_no_matches \
            "Rockchip upgrade_tool 命令码泄漏到通用层或前端" \
            '\b(LD|DB|DI|WL|UF|EF|RID|RFI|RCI|RCB|PL|RSN|RSM|TD|RD|SSD|SN|RP|UL|RL|EL|RCL)\b' \
            app/api/src app/config/src app/backend/src/domain \
            app/backend/src/application app/frontend/src
        packaging_assert_no_matches \
            "API 契约反向依赖了前端或后端实现" \
            'rkflash[_-](backend|frontend)|eframe|egui|rfd|libc' \
            app/api
        packaging_assert_no_matches \
            "config crate 反向依赖了 API、后端或前端" \
            'rkflash[_-](api|backend|frontend)|eframe|egui|rfd|libc' \
            app/config
        packaging_assert_no_matches \
            "backend crate 反向依赖了前端" \
            'rkflash[_-]frontend' \
            app/backend
        packaging_assert_no_matches \
            "frontend crate 直接依赖了 backend 实现" \
            'rkflash[_-]backend' \
            app/frontend
        packaging_assert_no_matches \
            "app crate 包含了业务、设备或界面实现" \
            'mod (application|domain|infrastructure|desktop)|upgrade_tool|FlashOperation' \
            app/launcher/src
    )
    echo "架构边界检查通过"
}

project_validate_package_inputs() {
    [[ -f "$UPGRADE_TOOL" ]] || \
        packaging_die "缺少 $UPGRADE_TOOL，请先放入 Rockchip 官方工具"
    [[ -f "$RUNTIME_DEPENDENCIES_FILE" ]] || \
        packaging_die "缺少 $RUNTIME_DEPENDENCIES_FILE"
    [[ -x "$DEBIAN_INSTALLER_TEMPLATE" ]] || \
        packaging_die "缺少可执行安装器模板 $DEBIAN_INSTALLER_TEMPLATE"
    (
        cd "$PROJECT_DIR" || packaging_die "无法进入项目目录：$PROJECT_DIR"
        sha256sum --check packaging/upgrade_tool.sha256
    )
}

project_validate_binary() {
    packaging_check_elf_glibc \
        "$RELEASE_BINARY" \
        "$PACKAGE_ARCHITECTURE" \
        "$GLIBC_BASELINE" \
        "$SUPPORTED_TARGETS"
    packaging_check_elf_glibc \
        "$UPGRADE_TOOL" \
        "$PACKAGE_ARCHITECTURE" \
        "$GLIBC_BASELINE" \
        "$SUPPORTED_TARGETS"
}

project_stage_payload() {
    local stage_dir=$1

    install -D -m 0755 "$RELEASE_BINARY" \
        "$stage_dir/usr/bin/$PACKAGE_NAME"
    install -D -m 0755 "$UPGRADE_TOOL" \
        "$stage_dir/usr/lib/$PACKAGE_NAME/upgrade_tool"
    install -D -m 0644 "$PROJECT_DIR/packaging/71-rkflash.rules" \
        "$stage_dir/usr/lib/udev/rules.d/71-rkflash.rules"
    install -D -m 0644 "$PROJECT_DIR/packaging/rkflash.desktop" \
        "$stage_dir/usr/share/applications/rkflash.desktop"
    local icon_size
    for icon_size in 48 64 128 256 512; do
        install -D -m 0644 "$PROJECT_DIR/packaging/icons/rkflash-$icon_size.png" \
            "$stage_dir/usr/share/icons/hicolor/${icon_size}x${icon_size}/apps/rkflash.png"
    done
    install -D -m 0644 "$PROJECT_DIR/README.md" \
        "$stage_dir/usr/share/doc/$PACKAGE_NAME/README.md"
    install -D -m 0644 "$PROJECT_DIR/packaging/THIRD_PARTY.md" \
        "$stage_dir/usr/share/doc/$PACKAGE_NAME/THIRD_PARTY.md"
    install -D -m 0644 "$PROJECT_DIR/LICENSE" \
        "$stage_dir/usr/share/doc/$PACKAGE_NAME/copyright"
    strip "$stage_dir/usr/bin/$PACKAGE_NAME"
    desktop-file-validate "$stage_dir/usr/share/applications/rkflash.desktop"
}

project_stage_maintainer_scripts() {
    local stage_dir=$1

    cat >"$stage_dir/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --action=add --subsystem-match=usb --attr-match=idVendor=2207 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q || true
fi
exit 0
EOF
    chmod 0755 "$stage_dir/DEBIAN/postinst"
    cat >"$stage_dir/DEBIAN/postrm" <<'EOF'
#!/bin/sh
# 升级/消失时新包的 postinst 会自行刷新，这里只在真正移除时收尾。
set -e
case "${1:-}" in
    remove|purge)
        if command -v udevadm >/dev/null 2>&1; then
            udevadm control --reload-rules || true
            udevadm trigger --action=add --subsystem-match=usb --attr-match=idVendor=2207 || true
        fi
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database -q || true
        fi
        ;;
esac
exit 0
EOF
    chmod 0755 "$stage_dir/DEBIAN/postrm"
}
