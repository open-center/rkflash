## 下载

| 平台 | 文件 | 安装 |
|---|---|---|
| Ubuntu 22.04 / 24.04 / 26.04（amd64） | `rktools_<版本>_amd64.deb` + `rktools-linux-install.sh` | 两个文件放同一目录，执行 `./rktools-linux-install.sh` |
| macOS 13+ Apple Silicon | `RKFlashMac-<版本>-arm64.dmg` | 打开后拖入“应用程序” |
| macOS 13+ Intel | `RKFlashMac-<版本>-x86_64.dmg` | 打开后拖入“应用程序” |

校验：`sha256sum --check SHA256SUMS`（macOS 用 `shasum -a 256 -c SHA256SUMS`）。

## 注意

- macOS 应用为临时签名，未经 Apple 公证。首次打开如被拦截：在 Finder 中右键应用 →“打开”，
  或执行 `xattr -dr com.apple.quarantine /Applications/RKFlashMac.app`。
- Linux 安装包内含 Rockchip 闭源 `upgrade_tool`（来自 vicharak-in/Linux_Upgrade_Tool，已按 SHA-256 校验），
  不属于本项目开源许可范围，详见 `linux/packaging/THIRD_PARTY.md`。
