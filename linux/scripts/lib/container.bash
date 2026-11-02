# 通用 Docker 构建与运行基础设施。项目镜像、版本矩阵和验收路径由配置钩子提供。

container_encode_certificate() {
    local certificate=$1 variable_name=$2 encoded

    if [[ -z "$certificate" ]]; then
        printf -v "$variable_name" '%s' ''
        return
    fi
    [[ -f "$certificate" && -r "$certificate" ]] || \
        packaging_die "无法读取证书：$certificate"
    (($(stat -c '%s' "$certificate") <= 32768)) || \
        packaging_die "只接受单张 PEM 证书（不超过 32 KiB）：$certificate"
    encoded=$(base64 --wrap=0 "$certificate")
    printf -v "$variable_name" '%s' "$encoded"
}

container_require_engine() {
    packaging_require_command docker
    docker info >/dev/null 2>&1 || \
        packaging_die "无法连接 Docker daemon；请启动 Docker 或配置当前用户的 Docker 权限"
}

container_ensure_builder_image() {
    local version=$1 role=$2 rebuild_policy=${3:-normal}
    local base_image apt_mirror builder_image environment_name build_image=0
    local -a build_args

    base_image=$(project_delivery_base_image "$version" "$role")
    apt_mirror=$(project_delivery_apt_mirror "$version" "$role")
    builder_image=$(project_delivery_builder_image "$version" "$role")
    project_delivery_prepare_build_arguments \
        "$version" "$base_image" "$apt_mirror" \
        "$DELIVERY_CUSTOM_CA_B64" "$DELIVERY_EXTRA_CA_B64"
    build_args=(
        build
        --platform "$DELIVERY_DOCKER_PLATFORM"
        "${PROJECT_DELIVERY_BUILD_ARGUMENTS[@]}"
        --file "$DELIVERY_DOCKERFILE"
        --tag "$builder_image"
    )
    for environment_name in \
        HTTP_PROXY HTTPS_PROXY NO_PROXY http_proxy https_proxy no_proxy; do
        if [[ -v "$environment_name" ]]; then
            build_args+=(--build-arg "$environment_name")
        fi
    done
    case "$rebuild_policy" in
        normal) ;;
        pull) build_args+=(--pull) ; build_image=1 ;;
        no-cache) build_args+=(--pull --no-cache) ; build_image=1 ;;
        *) packaging_die "未知镜像重建策略：$rebuild_policy" ;;
    esac
    build_args+=("$PROJECT_DIR")

    if ! docker image inspect "$builder_image" >/dev/null 2>&1; then
        build_image=1
    fi
    if [[ "$DELIVERY_FORCE_REBUILD" == 1 ]]; then
        build_image=1
    fi
    if ((build_image == 0)); then
        echo "[Docker] 复用已有构建镜像：$builder_image"
        return
    fi

    echo "[Docker] 准备构建环境：$base_image / $DELIVERY_DOCKER_PLATFORM"
    if [[ -n "$apt_mirror" ]]; then
        echo "[Docker] 系统 APT 镜像：$apt_mirror"
    fi
    docker "${build_args[@]}" || \
        packaging_die "构建镜像失败；网络受限时请配置对应版本的基础镜像和 APT 镜像"
}

container_run_builder() {
    local version=$1 role=$2 container_command=$3
    shift 3
    local builder_image apt_mirror environment_name xauthority_file cargo_target_dir
    local -a run_args command_args

    builder_image=$(project_delivery_builder_image "$version" "$role")
    apt_mirror=$(project_delivery_apt_mirror "$version" "$role")
    cargo_target_dir=$DELIVERY_DEFAULT_CARGO_TARGET_DIR
    if [[ "$container_command" == package ]]; then
        cargo_target_dir=$DELIVERY_PACKAGE_CARGO_TARGET_DIR
    fi
    command_args=("$container_command" "$@")
    run_args=(
        run
        --rm
        --init
        --platform "$DELIVERY_DOCKER_PLATFORM"
        --user "$(id -u):$(id -g)"
        --volume "$PROJECT_DIR:$DELIVERY_CONTAINER_WORKSPACE"
        --workdir "$DELIVERY_CONTAINER_WORKSPACE"
        --env "HOME=$DELIVERY_CONTAINER_HOME"
        --env "CARGO_HOME=$DELIVERY_CARGO_HOME"
        --env "CARGO_TARGET_DIR=$cargo_target_dir"
    )
    if [[ -t 0 ]]; then
        run_args+=(--interactive)
    fi
    if [[ -t 1 ]]; then
        run_args+=(--tty)
    fi
    for environment_name in "${DELIVERY_FORWARD_ENVIRONMENT[@]}"; do
        if [[ -v "$environment_name" ]]; then
            run_args+=(--env "$environment_name")
        fi
    done
    if [[ -n "$apt_mirror" ]]; then
        run_args+=(--env "OFFLINE_APT_MIRROR=$apt_mirror")
    elif [[ -v OFFLINE_APT_MIRROR ]]; then
        run_args+=(--env OFFLINE_APT_MIRROR)
    fi
    case "$container_command" in
        package)
            run_args+=(
                --env "DELIVERY_BASELINE_DISTRIBUTION=$DELIVERY_DISTRIBUTION"
                --env "DELIVERY_BASELINE_VERSION=$DELIVERY_BASELINE_VERSION"
            )
            ;;
        offline-bundle)
            run_args+=(
                --env "OFFLINE_TARGET_DISTRIBUTION=$DELIVERY_DISTRIBUTION"
                --env "OFFLINE_TARGET_VERSION=$version"
            )
            ;;
    esac

    if [[ "$container_command" == run ]]; then
        if [[ -n "${DISPLAY:-}" && -d /tmp/.X11-unix ]]; then
            run_args+=(--env DISPLAY --volume /tmp/.X11-unix:/tmp/.X11-unix:rw)
            xauthority_file=${XAUTHORITY:-${HOME:-}/.Xauthority}
            if [[ -n "$xauthority_file" && -f "$xauthority_file" ]]; then
                run_args+=(
                    --env XAUTHORITY=/tmp/delivery-Xauthority
                    --volume "$xauthority_file:/tmp/delivery-Xauthority:ro"
                )
            fi
        fi
        if [[ -n "${XDG_RUNTIME_DIR:-}" && -d "$XDG_RUNTIME_DIR" ]]; then
            # 只挂载确需的 Wayland/DBus socket 文件，而不是整个运行时目录；
            # 整目录挂载会把 SSH agent、PulseAudio 等其他会话服务一并暴露给容器。
            if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
                if [[ "$WAYLAND_DISPLAY" == /* ]]; then
                    wayland_socket=$WAYLAND_DISPLAY
                else
                    wayland_socket=$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY
                fi
                if [[ -S "$wayland_socket" ]]; then
                    run_args+=(
                        --env XDG_RUNTIME_DIR
                        --env WAYLAND_DISPLAY
                        --volume "$wayland_socket:$wayland_socket:rw"
                    )
                fi
            fi
            if [[ "${DBUS_SESSION_BUS_ADDRESS:-}" == unix:path=* ]]; then
                dbus_socket=${DBUS_SESSION_BUS_ADDRESS#unix:path=}
                if [[ -S "$dbus_socket" ]]; then
                    run_args+=(
                        --env DBUS_SESSION_BUS_ADDRESS
                        --volume "$dbus_socket:$dbus_socket:rw"
                    )
                fi
            fi
        fi
        if [[ -d /dev/bus/usb ]]; then
            run_args+=(
                --volume /dev/bus/usb:/dev/bus/usb:rw
                --device-cgroup-rule "c 189:* rmw"
            )
        fi
    fi

    echo "[Docker] $DELIVERY_DISTRIBUTION_DISPLAY $version 执行：./build.sh ${command_args[*]}"
    docker "${run_args[@]}" "$builder_image" "${command_args[@]}"
}

container_verify_offline_bundle() {
    local version=$1 role=$2 archive_name=$3
    local base_image bundle_name verify_script verify_executables verify_files

    base_image=$(project_delivery_base_image "$version" "$role")
    bundle_name=${archive_name%.tar.gz}
    [[ -f "$DIST_DIR/$archive_name" ]] || \
        packaging_die "缺少待验证的离线包：$DIST_DIR/$archive_name"
    [[ -f "$DIST_DIR/$archive_name.sha256" ]] || \
        packaging_die "缺少离线包校验文件：$DIST_DIR/$archive_name.sha256"
    docker image inspect "$base_image" >/dev/null 2>&1 || \
        packaging_die "缺少离线验证所需的基础镜像：$base_image"
    printf -v verify_executables '%s\n' "${DELIVERY_VERIFY_EXECUTABLES[@]}"
    printf -v verify_files '%s\n' "${DELIVERY_VERIFY_FILES[@]}"

    # The script is expanded only by the isolated verification container.
    # shellcheck disable=SC2016
    verify_script='
set -euo pipefail
cd /delivery
sha256sum --check "$ARCHIVE_NAME.sha256"
verification_root=$(mktemp -d /tmp/offline-verification.XXXXXX)
tar -xzf "$ARCHIVE_NAME" -C "$verification_root"
cd "$verification_root/$BUNDLE_NAME"
sha256sum --check SHA256SUMS
./install.sh
dpkg-query -s "$PACKAGE_NAME" | grep -Fx "Status: install ok installed"
while IFS= read -r required_path; do
    [[ -z "$required_path" ]] || test -x "$required_path"
done <<<"$VERIFY_EXECUTABLES"
while IFS= read -r required_path; do
    [[ -z "$required_path" ]] || test -f "$required_path"
done <<<"$VERIFY_FILES"
'
    echo "[验证] 在干净 $DELIVERY_DISTRIBUTION_DISPLAY $version 容器中断网安装离线包"
    docker run --rm --network none --platform "$DELIVERY_DOCKER_PLATFORM" \
        --volume "$DIST_DIR:/delivery:ro" \
        --env "ARCHIVE_NAME=$archive_name" \
        --env "BUNDLE_NAME=$bundle_name" \
        --env "PACKAGE_NAME=$PACKAGE_NAME" \
        --env "VERIFY_EXECUTABLES=$verify_executables" \
        --env "VERIFY_FILES=$verify_files" \
        --entrypoint /bin/bash \
        "$base_image" -lc "$verify_script" || \
        packaging_die "$DELIVERY_DISTRIBUTION_DISPLAY $version 离线安装验证失败"
    echo "[验证] $DELIVERY_DISTRIBUTION_DISPLAY $version 离线安装通过"
}
