#!/usr/bin/env bash
# 构建 WinUSB 驱动安装工具 wdi-simple.exe（libwdi，与 Zadig 相同的安装机制）。
#
# libwdi 在目标机上现场生成驱动 INF/CAT，并用临时自签名证书签名，因此无需购买驱动签名证书。
# 驱动本身是 Windows 自带的 WinUSB；构建时需要嵌入微软 WDK 8.0 再分发的 WinUSB/KMDF
# CoInstaller（只在 Windows 7 上实际使用）。两个下载都固定版本并校验 SHA-256。
#
# 用法：windows/driver/build.sh [输出目录]
#   CROSS=x86_64-w64-mingw32-  交叉编译前缀（MSYS2 原生构建时留空）
#   CC_FOR_BUILD=cc            运行在构建机上的编译器（用于 libwdi 的 embedder）

set -euo pipefail

HERE=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
BUILD_DIR=$(cd -- "$HERE/.." && pwd)/build/driver
OUTPUT_DIR=${1:-$BUILD_DIR/bin}
CROSS=${CROSS:-}
CC_FOR_BUILD=${CC_FOR_BUILD:-cc}
SEVEN_ZIP=${SEVEN_ZIP:-$(command -v 7zz || command -v 7z || true)}

LIBWDI_VERSION=1.5.1
LIBWDI_URL="https://github.com/pbatard/libwdi/archive/refs/tags/v${LIBWDI_VERSION}.tar.gz"
LIBWDI_SHA256=a695e93db0977dfdc5c6a99a4ea91b22f9027547d0177b2a0f3075078643c929
WDF_MSI_URL="https://go.microsoft.com/fwlink/p/?LinkID=253170"
WDF_MSI_SHA256=29314207814ce9d5d73695f7e9239539cf37c79e750b9d5ea5a5ef5487a583d6

die() { echo "错误：$*" >&2; exit 1; }

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

fetch() {
    local url=$1 file=$2 expected=$3
    if [[ ! -f "$file" ]] || [[ "$(sha256 "$file")" != "$expected" ]]; then
        curl -fsSL --retry 3 -o "$file.part" "$url"
        mv "$file.part" "$file"
    fi
    [[ "$(sha256 "$file")" == "$expected" ]] || die "$(basename "$file") 校验失败"
}

[[ -n "$SEVEN_ZIP" ]] || die "需要 7-Zip（7z 或 7zz）解开 WDK 再分发包"
mkdir -p "$BUILD_DIR/downloads" "$OUTPUT_DIR"
fetch "$LIBWDI_URL" "$BUILD_DIR/downloads/libwdi-$LIBWDI_VERSION.tar.gz" "$LIBWDI_SHA256"
fetch "$WDF_MSI_URL" "$BUILD_DIR/downloads/wdfcoinstaller.msi" "$WDF_MSI_SHA256"

SOURCE="$BUILD_DIR/libwdi-$LIBWDI_VERSION"
rm -rf "$SOURCE" "$BUILD_DIR/wdk" "$BUILD_DIR/wdk-msi"
tar -xzf "$BUILD_DIR/downloads/libwdi-$LIBWDI_VERSION.tar.gz" -C "$BUILD_DIR"

# MSI 内同名文件的 _00000 为 x86 版，_00001 为 x64 版；整理成 libwdi 期望的 WDK 目录结构。
"$SEVEN_ZIP" x -y -o"$BUILD_DIR/wdk-msi" "$BUILD_DIR/downloads/wdfcoinstaller.msi" >/dev/null
for arch in x86:00000 x64:00001; do
    name=${arch%%:*}
    suffix=${arch##*:}
    install -d "$BUILD_DIR/wdk/redist/wdf/$name" "$BUILD_DIR/wdk/redist/winusb/$name"
    cp "$BUILD_DIR/wdk-msi/_WdfCoInstaller01011.dll_$suffix" "$BUILD_DIR/wdk/redist/wdf/$name/WdfCoInstaller01011.dll"
    cp "$BUILD_DIR/wdk-msi/_winusbcoinstaller2.dll_$suffix" "$BUILD_DIR/wdk/redist/winusb/$name/winusbcoinstaller2.dll"
done
install -m 0644 "$BUILD_DIR/wdk-msi/License.rtf" "$OUTPUT_DIR/WDK-Redist-License.rtf"

cat >"$SOURCE/config.h" <<EOF
/* rkflash: libwdi 配置，仅嵌入 x64 WinUSB 驱动支持 */
#define WDK_DIR "$BUILD_DIR/wdk"
#define WDF_VER 1011
#define COINSTALLER_DIR "winusb"
#define X64_DIR "x64"
#define OPT_M64
#define ENABLE_LOGGING 1
EOF

DEFINES=(-DWINVER=0x0601 -D_WIN32_WINNT=0x0601 -D_WIN32_IE=0x0700 -DCOBJMACROS -D_WIN64)
CFLAGS=(-O2 -m64 -I"$SOURCE" -I"$SOURCE/libwdi" "${DEFINES[@]}" -Wno-stringop-truncation)

cd "$SOURCE/libwdi"
"${CROSS}gcc" "${CFLAGS[@]}" installer.c -o installer_x64.exe -static -lsetupapi -lnewdev -lole32
"$CC_FOR_BUILD" -I"$SOURCE" embedder.c -o embedder
./embedder embedded.h

objects=()
for source in logging tokenizer vid_data pki libwdi_dlg libwdi; do
    "${CROSS}gcc" "${CFLAGS[@]}" -c "$source.c" -o "$source.o"
    objects+=("$source.o")
done
"${CROSS}windres" -I"$SOURCE/libwdi" libwdi.rc -O coff -o libwdi_rc.o
objects+=(libwdi_rc.o)

cd "$SOURCE/examples"
# MinGW 自带 getopt_long，不使用 libwdi 附带的 getopt 副本（其声明与 MinGW 冲突）。
"${CROSS}gcc" "${CFLAGS[@]}" -c wdi-simple.c -o wdi-simple.o
"${CROSS}windres" -I"$SOURCE/examples" wdi-simple.rc -O coff -o wdi-simple_rc.o
"${CROSS}gcc" -m64 wdi-simple.o wdi-simple_rc.o \
    "${objects[@]/#/$SOURCE/libwdi/}" \
    -o "$OUTPUT_DIR/wdi-simple.exe" -static -lsetupapi -lole32 -lntdll -lnewdev -lshell32 -luuid
"${CROSS}strip" "$OUTPUT_DIR/wdi-simple.exe"
install -m 0644 "$SOURCE/COPYING" "$OUTPUT_DIR/libwdi-COPYING.txt"
echo "已生成：$OUTPUT_DIR/wdi-simple.exe"
