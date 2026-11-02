# 通用交付工作流。这里只编排生命周期，不包含项目名、发行版矩阵或镜像地址。

delivery_validate_version() {
    local version=$1
    [[ "$version" =~ ^[0-9]{2}\.[0-9]{2}$ ]] || \
        packaging_die "发行版版本格式无效：$version"
}

delivery_require_supported_target() {
    local version=$1 supported

    for supported in "${DELIVERY_TARGET_VERSIONS[@]}"; do
        [[ "$version" == "$supported" ]] && return 0
    done
    packaging_die \
        "离线交付只支持 $DELIVERY_DISTRIBUTION_DISPLAY ${DELIVERY_TARGET_VERSIONS[*]}，检测到 $version"
}

delivery_validate_configuration() {
    local hook

    [[ -n "$PACKAGE_NAME" && -n "$PACKAGE_VERSION" && -n "$PACKAGE_ARCHITECTURE" ]] || \
        packaging_die "项目包配置不完整"
    [[ -n "$DELIVERY_DISTRIBUTION" && -n "$DELIVERY_BASELINE_VERSION" ]] || \
        packaging_die "交付基线配置不完整"
    [[ -x "$DEBIAN_INSTALLER_TEMPLATE" ]] || \
        packaging_die "缺少可执行安装器模板 $DEBIAN_INSTALLER_TEMPLATE"
    ((${#DELIVERY_TARGET_VERSIONS[@]} > 0)) || packaging_die "交付目标矩阵为空"
    [[ -f "$DELIVERY_DOCKERFILE" ]] || packaging_die "缺少 $DELIVERY_DOCKERFILE"
    for hook in \
        project_delivery_base_image project_delivery_apt_mirror \
        project_delivery_builder_image project_delivery_prepare_build_arguments; do
        declare -F "$hook" >/dev/null || packaging_die "缺少项目交付钩子：$hook"
    done
    delivery_validate_version "$DELIVERY_BASELINE_VERSION"
    delivery_validate_version "$DELIVERY_REQUESTED_VERSION"
}

delivery_offline_archive_name() {
    local version=$1
    printf '%s_%s_%s-%s_%s_offline.tar.gz' \
        "$PACKAGE_NAME" "$PACKAGE_VERSION" "$DELIVERY_DISTRIBUTION" \
        "$version" "$PACKAGE_ARCHITECTURE"
}

delivery_package_filename() {
    printf '%s_%s_%s.deb' "$PACKAGE_NAME" "$PACKAGE_VERSION" "$PACKAGE_ARCHITECTURE"
}

delivery_install_package() (
    local package_file package_filename

    package_filename=$(delivery_package_filename)
    package_file="$DIST_DIR/$package_filename"
    [[ -f "$package_file" ]] || packaging_die \
        "缺少 $package_file；请先执行 ./build.sh build"
    "$DEBIAN_INSTALLER_TEMPLATE" "$package_file"
)

delivery_clean() {
    packaging_remove_output_dir "$PROJECT_DIR" "$OUTPUT_ROOT"
    echo "[产出] 已清理：$OUTPUT_ROOT"
}

delivery_prepare_host() {
    container_require_engine
    delivery_validate_configuration
    container_encode_certificate "$DELIVERY_CUSTOM_CA_CERT" DELIVERY_CUSTOM_CA_B64
    container_encode_certificate "$DELIVERY_EXTRA_CA_CERT" DELIVERY_EXTRA_CA_B64
}

delivery_build_package() {
    local role=${1:-primary}

    echo "[交付] .deb 固定使用 $DELIVERY_DISTRIBUTION_DISPLAY $DELIVERY_BASELINE_VERSION 兼容基线构建"
    container_ensure_builder_image "$DELIVERY_BASELINE_VERSION" "$role" normal
    container_run_builder "$DELIVERY_BASELINE_VERSION" "$role" package
}

delivery_build_offline_target() {
    local version=$1 role=$2 archive_name

    echo "[交付] 解析 $DELIVERY_DISTRIBUTION_DISPLAY $version 专属离线依赖"
    container_ensure_builder_image "$version" "$role" normal
    container_run_builder "$version" "$role" offline-bundle
    archive_name=$(delivery_offline_archive_name "$version")
    container_verify_offline_bundle "$version" "$role" "$archive_name"
}

delivery_dispatch() {
    local command_name=${1:-build}
    local baseline_role target_role target_version

    case "$command_name" in
        clean)
            (($# == 1)) || packaging_die "clean 不接受其他参数"
            delivery_clean
            return
            ;;
        install)
            (($# == 1)) || packaging_die "install 不接受其他参数"
            delivery_install_package
            return
            ;;
    esac

    delivery_prepare_host
    case "$command_name" in
        build)
            (($# <= 1)) || packaging_die "build 不接受其他参数"
            delivery_build_package primary
            ;;
        package)
            (($# == 1)) || packaging_die "package 不接受其他参数"
            delivery_build_package primary
            ;;
        offline)
            (($# == 1)) || packaging_die "offline 不接受其他参数"
            delivery_require_supported_target "$DELIVERY_REQUESTED_VERSION"
            baseline_role=baseline
            if [[ "$DELIVERY_REQUESTED_VERSION" == "$DELIVERY_BASELINE_VERSION" ]]; then
                baseline_role=primary
            fi
            delivery_build_package "$baseline_role"
            delivery_build_offline_target "$DELIVERY_REQUESTED_VERSION" primary
            ;;
        offline-all)
            (($# == 1)) || packaging_die "offline-all 不接受其他参数"
            delivery_build_package baseline
            for target_version in "${DELIVERY_TARGET_VERSIONS[@]}"; do
                target_role=target
                if [[ "$target_version" == "$DELIVERY_BASELINE_VERSION" ]]; then
                    target_role=baseline
                fi
                delivery_build_offline_target "$target_version" "$target_role"
            done
            ;;
        setup)
            container_ensure_builder_image "$DELIVERY_REQUESTED_VERSION" primary pull
            container_run_builder "$DELIVERY_REQUESTED_VERSION" primary "$@"
            ;;
        toolchain)
            container_ensure_builder_image "$DELIVERY_REQUESTED_VERSION" primary no-cache
            container_run_builder "$DELIVERY_REQUESTED_VERSION" primary "$@"
            ;;
        *)
            container_ensure_builder_image "$DELIVERY_REQUESTED_VERSION" primary normal
            if (($# == 0)); then
                container_run_builder "$DELIVERY_REQUESTED_VERSION" primary build
            else
                container_run_builder "$DELIVERY_REQUESTED_VERSION" primary "$@"
            fi
            ;;
    esac
}
