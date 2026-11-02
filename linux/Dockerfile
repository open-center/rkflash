ARG UBUNTU_VERSION=22.04
ARG UBUNTU_IMAGE=ubuntu:${UBUNTU_VERSION}
FROM ${UBUNTU_IMAGE}

ARG DEBIAN_FRONTEND=noninteractive
ARG UBUNTU_APT_MIRROR=
ARG CUSTOM_CA_CERT_B64=
ARG EXTRA_CA_CERT_B64=
# Explicit declarations also support Docker's legacy builder, which otherwise
# does not consistently expose the predefined proxy build arguments to RUN.
ARG HTTP_PROXY
ARG HTTPS_PROXY
ARG NO_PROXY
ARG http_proxy
ARG https_proxy
ARG no_proxy

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Keep both build and packaging dependencies in the image. The APT indexes are
# intentionally retained because the offline-bundle command uses them to solve
# the dependency closure for the selected Ubuntu release.
# The clean dpkg status is snapshotted before any build dependency is installed
# so the offline closure is solved against a pristine baseline system.
# ca-certificates is bootstrapped from the image's original (plain http)
# sources first: some base image variants ship without a CA bundle, which would
# otherwise break every https mirror or rustup download below.
RUN set -e; \
    cp /var/lib/dpkg/status /var/lib/dpkg/status.clean-base; \
    apt-get \
        -o Acquire::http::No-Cache=true \
        -o Acquire::https::No-Cache=true \
        -o Acquire::Retries=3 \
        update; \
    apt-get \
        -o Acquire::http::No-Cache=true \
        -o Acquire::https::No-Cache=true \
        -o Acquire::Retries=3 \
        install -y --no-install-recommends ca-certificates; \
    if [[ -n "$CUSTOM_CA_CERT_B64" || -n "$EXTRA_CA_CERT_B64" ]]; then \
        mkdir -p /usr/local/share/ca-certificates; \
        if [[ -n "$CUSTOM_CA_CERT_B64" ]]; then \
            printf '%s' "$CUSTOM_CA_CERT_B64" | base64 --decode \
                >/usr/local/share/ca-certificates/rkflash-build-ca.crt; \
        fi; \
        if [[ -n "$EXTRA_CA_CERT_B64" ]]; then \
            printf '%s' "$EXTRA_CA_CERT_B64" | base64 --decode \
                >/usr/local/share/ca-certificates/rkflash-extra-ca.crt; \
        fi; \
        update-ca-certificates; \
    fi; \
    if [[ -n "$UBUNTU_APT_MIRROR" ]]; then \
        apt_mirror=${UBUNTU_APT_MIRROR%/}; \
        [[ "$apt_mirror" == */ubuntu ]] || apt_mirror=$apt_mirror/ubuntu; \
        for source_file in /etc/apt/sources.list /etc/apt/sources.list.d/*.sources; do \
            [[ -f "$source_file" ]] || continue; \
            sed -i -E \
                "s@https?://(archive|security)\.ubuntu\.com/ubuntu@$apt_mirror@g" \
                "$source_file"; \
        done; \
        apt-get \
            -o Acquire::http::No-Cache=true \
            -o Acquire::https::No-Cache=true \
            -o Acquire::Retries=3 \
            update; \
    fi \
    && apt-get \
        -o Acquire::http::No-Cache=true \
        -o Acquire::https::No-Cache=true \
        -o Acquire::Retries=3 \
        install -y --no-install-recommends \
        binutils \
        build-essential \
        ca-certificates \
        curl \
        desktop-file-utils \
        dpkg-dev \
        file \
        libgl1-mesa-dev \
        libudev-dev \
        libusb-1.0-0-dev \
        libwayland-dev \
        libx11-dev \
        libxi-dev \
        libxkbcommon-dev \
        pkg-config \
        ripgrep \
    && rm -rf /var/cache/apt/archives/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH} \
    CONTAINERIZED_BUILD=1 \
    RUST_BOOTSTRAP_DISABLED=1 \
    SYSTEM_BOOTSTRAP_DISABLED=1 \
    BASELINE_DPKG_STATUS=/var/lib/dpkg/status.clean-base

COPY rust-toolchain.toml /tmp/rkflash-rust-toolchain.toml

RUN rust_channel=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' /tmp/rkflash-rust-toolchain.toml) \
    && [[ -n "$rust_channel" ]] \
    && curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
        https://sh.rustup.rs --output /tmp/rustup-init.sh \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "$rust_channel" \
        --component clippy --component rustfmt --no-modify-path \
    && rm -f /tmp/rustup-init.sh /tmp/rkflash-rust-toolchain.toml \
    && rustc --version \
    && cargo --version \
    && cargo clippy --version \
    && rustfmt --version

COPY scripts/docker-entrypoint.sh /usr/local/bin/rkflash-container-entrypoint
RUN chmod 0755 /usr/local/bin/rkflash-container-entrypoint

WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/rkflash-container-entrypoint"]
