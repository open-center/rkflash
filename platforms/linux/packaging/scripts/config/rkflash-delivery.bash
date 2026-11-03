# rkflash 交付矩阵与 Docker 策略。通用编排层只通过这里的变量和钩子感知项目差异。
# shellcheck disable=SC2034

project_delivery_configure() {
    local root_dir=$1

    DELIVERY_DISTRIBUTION=ubuntu
    DELIVERY_DISTRIBUTION_DISPLAY=Ubuntu
    DELIVERY_BASELINE_VERSION=22.04
    DELIVERY_TARGET_VERSIONS=(22.04 24.04 26.04)
    SUPPORTED_TARGETS="$DELIVERY_DISTRIBUTION_DISPLAY ${DELIVERY_TARGET_VERSIONS[*]}"
    DELIVERY_REQUESTED_VERSION=${RKFLASH_UBUNTU_VERSION:-$DELIVERY_BASELINE_VERSION}
    DELIVERY_DOCKER_PLATFORM=${RKFLASH_DOCKER_PLATFORM:-linux/amd64}
    DELIVERY_DOCKERFILE="$root_dir/platforms/linux/packaging/Dockerfile"
    DELIVERY_CONTAINER_WORKSPACE=/workspace
    DELIVERY_CONTAINER_HOME=/tmp/delivery-home
    DELIVERY_CARGO_HOME=/workspace/output/docker/cargo-home
    DELIVERY_DEFAULT_CARGO_TARGET_DIR=/workspace/output/target
    DELIVERY_PACKAGE_CARGO_TARGET_DIR="/workspace/output/target/delivery-${DELIVERY_DISTRIBUTION}-${DELIVERY_BASELINE_VERSION}"
    DELIVERY_CUSTOM_CA_CERT=${RKFLASH_DOCKER_CA_CERT:-}
    DELIVERY_EXTRA_CA_CERT=${RKFLASH_DOCKER_EXTRA_CA_CERT:-}
    DELIVERY_FORCE_REBUILD=${RKFLASH_DOCKER_REBUILD:-0}
    DELIVERY_FORWARD_ENVIRONMENT=(
        HTTP_PROXY HTTPS_PROXY NO_PROXY http_proxy https_proxy no_proxy
        CARGO_NET_OFFLINE CARGO_NET_GIT_FETCH_WITH_CLI RUST_BACKTRACE
        RKFLASH_LOG_DIR RK_UPGRADE_TOOL SOURCE_DATE_EPOCH
    )
    DELIVERY_VERIFY_EXECUTABLES=(
        "/usr/bin/$PACKAGE_NAME"
        "/usr/lib/$PACKAGE_NAME/upgrade_tool"
    )
    DELIVERY_VERIFY_FILES=(
        "/usr/lib/udev/rules.d/71-rkflash.rules"
        "/usr/share/applications/rkflash.desktop"
    )
}

project_delivery_version_setting() {
    local setting_name=$1 version=$2 variable_name

    variable_name="RKFLASH_${setting_name}_${version//./_}"
    if [[ -v "$variable_name" ]]; then
        printf '%s' "${!variable_name}"
    fi
}

project_delivery_base_image() {
    local version=$1 role=$2 configured

    configured=$(project_delivery_version_setting UBUNTU_IMAGE "$version")
    if [[ -n "$configured" ]]; then
        printf '%s' "$configured"
    elif [[ "$role" == baseline && -n "${RKFLASH_BASELINE_UBUNTU_IMAGE:-}" ]]; then
        printf '%s' "$RKFLASH_BASELINE_UBUNTU_IMAGE"
    elif [[ "$role" == primary && -n "${RKFLASH_UBUNTU_IMAGE:-}" ]]; then
        printf '%s' "$RKFLASH_UBUNTU_IMAGE"
    else
        printf '%s:%s' "$DELIVERY_DISTRIBUTION" "$version"
    fi
}

project_delivery_apt_mirror() {
    local version=$1 role=$2 configured

    configured=$(project_delivery_version_setting UBUNTU_APT_MIRROR "$version")
    if [[ -n "$configured" ]]; then
        printf '%s' "$configured"
    elif [[ "$role" == baseline && -n "${RKFLASH_BASELINE_UBUNTU_APT_MIRROR:-}" ]]; then
        printf '%s' "$RKFLASH_BASELINE_UBUNTU_APT_MIRROR"
    else
        printf '%s' "${RKFLASH_UBUNTU_APT_MIRROR:-}"
    fi
}

project_delivery_builder_image() {
    local version=$1 role=$2 configured

    configured=$(project_delivery_version_setting DOCKER_IMAGE "$version")
    if [[ -n "$configured" ]]; then
        printf '%s' "$configured"
    elif [[ "$role" == baseline && -n "${RKFLASH_BASELINE_DOCKER_IMAGE:-}" ]]; then
        printf '%s' "$RKFLASH_BASELINE_DOCKER_IMAGE"
    elif [[ "$role" == primary && -n "${RKFLASH_DOCKER_IMAGE:-}" ]]; then
        printf '%s' "$RKFLASH_DOCKER_IMAGE"
    else
        printf '%s-builder:%s-%s' "$PACKAGE_NAME" "$DELIVERY_DISTRIBUTION" "$version"
    fi
}

project_delivery_prepare_build_arguments() {
    local version=$1 base_image=$2 apt_mirror=$3 custom_ca_b64=$4 extra_ca_b64=$5

    PROJECT_DELIVERY_BUILD_ARGUMENTS=(
        --build-arg "UBUNTU_VERSION=$version"
        --build-arg "UBUNTU_IMAGE=$base_image"
        --build-arg "UBUNTU_APT_MIRROR=$apt_mirror"
        --build-arg "CUSTOM_CA_CERT_B64=$custom_ca_b64"
        --build-arg "EXTRA_CA_CERT_B64=$extra_ca_b64"
    )
}

project_default_package_mirror() {
    local distribution=$1

    if [[ "$distribution" == ubuntu ]]; then
        printf '%s' 'http://archive.ubuntu.com/ubuntu'
    fi
}
