#!/usr/bin/env bash

set -euo pipefail

die() {
    echo "错误：$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || die "缺少命令 $1"
}

for command_name in apt-get basename chmod dpkg-deb install mktemp readlink rm; do
    require_command "$command_name"
done

if (($# == 0)); then
    script_dir=$(cd -- "$(dirname -- "$(readlink -f -- "${BASH_SOURCE[0]}")")" && pwd)
    shopt -s nullglob
    package_candidates=("$script_dir"/*.deb)
    shopt -u nullglob
    ((${#package_candidates[@]} == 1)) || \
        die "安装器同目录必须且只能包含一个 .deb 文件"
    package_file=${package_candidates[0]}
elif (($# == 1)); then
    package_file=$1
else
    die "用法：$0 [软件包.deb]"
fi

[[ -f "$package_file" && -r "$package_file" ]] || die "无法读取软件包：$package_file"
package_file=$(readlink -f -- "$package_file")
package_name=$(dpkg-deb -f "$package_file" Package)
[[ "$package_name" =~ ^[a-z0-9][a-z0-9+.-]*$ ]] || \
    die "软件包名称无效：$package_name"
package_filename=$(basename -- "$package_file")

root_command=()
if ((EUID != 0)); then
    require_command sudo
    root_command=(sudo)
fi

staging_dir=
cleanup() {
    if [[ -n "$staging_dir" && "$staging_dir" == "/var/tmp/${package_name}-install."* ]]; then
        cleanup_target=$staging_dir
        staging_dir=
        "${root_command[@]}" rm -rf -- "$cleanup_target"
    fi
}
trap cleanup EXIT

staging_dir=$("${root_command[@]}" mktemp -d "/var/tmp/${package_name}-install.XXXXXX")
[[ "$staging_dir" == "/var/tmp/${package_name}-install."* ]] || \
    die "安装暂存目录不符合预期：$staging_dir"
"${root_command[@]}" chmod 0755 "$staging_dir"
"${root_command[@]}" install -m 0644 \
    "$package_file" "$staging_dir/$package_filename"

echo "[安装] 使用 APT 可读暂存目录：$staging_dir"
"${root_command[@]}" apt-get install -y --no-install-recommends \
    "$staging_dir/$package_filename"
echo "[安装] 完成。卸载命令：sudo apt purge $package_name"
