# Rust 工作区开发命令。此文件由根目录 build.sh 装配，不作为独立入口。

rust_prepare_toolchain() {
    debian_prepare_system_dependencies bootstrap
    packaging_prepare_rust_toolchain
}

rust_update_toolchain() {
    if [[ "${CONTAINERIZED_BUILD:-0}" == 1 ]]; then
        packaging_check_rust_toolchain
        echo "[环境] Rust、rustfmt 和 Clippy 已固化在 Docker 镜像中"
        return
    fi
    debian_prepare_system_dependencies bootstrap
    packaging_bootstrap_rust_toolchain
    packaging_load_cargo_environment
    hash -r
    packaging_select_rust_toolchain || packaging_die "更新后仍未找到 Rust 工具链"
    packaging_rust_version_is_supported "$RUST_SELECTED_VERSION" || packaging_die \
        "更新后的 Rust 版本仍过旧：$RUST_SELECTED_VERSION"
    echo "[环境] Rust 工具链已就绪：$(rustc --version)"
}

rust_build_release() {
    debian_prepare_system_dependencies development
    rust_prepare_toolchain
    cargo build --locked --release --workspace
}

rust_run() {
    debian_prepare_system_dependencies development
    rust_prepare_toolchain
    cargo run --locked --package "$APPLICATION_PACKAGE" -- "$@"
}

rust_format() {
    rust_prepare_toolchain
    cargo fmt --all -- --check
}

rust_test() {
    debian_prepare_system_dependencies development
    rust_prepare_toolchain
    cargo test --locked --workspace --all-targets
}

rust_lint() {
    debian_prepare_system_dependencies development
    rust_prepare_toolchain
    cargo clippy --locked --workspace --all-targets -- -D warnings
}

rust_check_architecture() {
    debian_prepare_system_dependencies architecture
    project_configure "$PROJECT_DIR"
    project_validate_sources
}

rust_check() {
    rust_format
    rust_check_architecture
    rust_test
    rust_lint
}

rust_clean() {
    packaging_remove_output_dir "$PROJECT_DIR" "$OUTPUT_ROOT"
}
