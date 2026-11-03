#!/usr/bin/env bash
# 仓库唯一版本来源是根目录 VERSION。本脚本把它同步到两个平台的工程文件，或检查是否一致。
#
#   scripts/version.sh              打印当前版本
#   scripts/version.sh check        检查 Rust 工作区与 macOS 是否与 VERSION 一致（CI 使用）
#   scripts/version.sh set X.Y.Z    写入 VERSION 并同步到两个平台

set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
VERSION_FILE="$ROOT/VERSION"
CARGO_TOML="$ROOT/Cargo.toml"
CARGO_LOCK="$ROOT/Cargo.lock"
INFO_PLIST="$ROOT/platforms/macos/Resources/Info.plist"
# Cargo.lock 中没有 source 字段的条目就是本工作区的 crate，版本随工作区一起变化；
# 按此规则识别，新增或拆分 crate 时无需修改本脚本。
WORKSPACE_ENTRY='\[\[package\]\]\nname = "(rkflash[\w-]*)"\nversion = "([^"]*)"\n(?!source)'

die() { echo "错误：$*" >&2; exit 1; }

read_version() {
    local v
    v=$(tr -d '[:space:]' <"$VERSION_FILE")
    [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "VERSION 格式应为 X.Y.Z，实际为 '$v'"
    echo "$v"
}

cargo_version() {
    sed -n '/^\[workspace\.package\]/,/^\[/ s/^version = "\([^"]*\)"/\1/p' "$CARGO_TOML" | head -n 1
}

lock_versions() {
    perl -0ne 'while (/'"$WORKSPACE_ENTRY"'/g) { print "$1=$2\n" }' "$CARGO_LOCK"
}

plist_version() {
    # 不依赖 PlistBuddy，Linux 上的 CI 也能检查
    perl -0ne 'print $1 if /<key>CFBundleShortVersionString<\/key>\s*<string>([^<]*)<\/string>/' "$INFO_PLIST"
}

check() {
    local v failed=0 line
    v=$(read_version)
    [[ "$(cargo_version)" == "$v" ]] || { echo "Cargo.toml: $(cargo_version) != $v"; failed=1; }
    while IFS= read -r line; do
        [[ "${line#*=}" == "$v" ]] || { echo "Cargo.lock: $line != $v"; failed=1; }
    done < <(lock_versions)
    [[ $(lock_versions | wc -l) -gt 0 ]] || { echo "Cargo.lock: 未找到工作区 crate"; failed=1; }
    [[ "$(plist_version)" == "$v" ]] || { echo "platforms/macos/Resources/Info.plist: $(plist_version) != $v"; failed=1; }
    ((failed == 0)) || die "版本不一致，请执行 scripts/version.sh set $v"
    echo "版本一致：$v"
}

set_version() {
    local v=$1
    [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "版本格式应为 X.Y.Z"
    echo "$v" >"$VERSION_FILE"
    perl -0pi -e 's/(\[workspace\.package\]\n(?:[^\[\n][^\n]*\n)*?version = ")[^"]*(")/${1}'"$v"'${2}/' "$CARGO_TOML"
    perl -0pi -e 's/(\[\[package\]\]\nname = "rkflash[\w-]*"\nversion = ")[^"]*("\n(?!source))/${1}'"$v"'${2}/g' "$CARGO_LOCK"
    perl -0pi -e 's/(<key>CFBundleShortVersionString<\/key>\s*<string>)[^<]*(<\/string>)/${1}'"$v"'${2}/' "$INFO_PLIST"
    check
}

case "${1:-}" in
    "") read_version ;;
    check) check ;;
    set) [[ $# -eq 2 ]] || die "用法：$0 set X.Y.Z"; set_version "$2" ;;
    *) die "用法：$0 [check | set X.Y.Z]" ;;
esac
